//! Host: un'istanza di Sunshine dedicata a Falco, viva solo quando si è «Disponibile» (§8.1).

use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{Context, bail};
use falco_engines::process::{Priority, command};
use falco_engines::sunshine::{self, GpuVendor, HostSettings, Platform};
use falco_engines::sunshine_api::SunshineApi;
use falco_store::config::{Config, Encoder};
use serde::Serialize;
use tokio::process::Child;

pub const API_USER: &str = "falco";

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase", tag = "state", content = "message")]
pub enum HostState {
    Stopped,
    Starting,
    Ready,
    Error(String),
}

pub struct Host {
    pub state: HostState,
    child: Option<Child>,
    api: Option<Arc<SunshineApi>>,
    gpu: Option<GpuVendor>,
    dir: PathBuf,
    /// Input accesi nell'istanza in esecuzione.
    pub inputs: sunshine::Inputs,
}

/// Cartella dei dati di Sunshine, creata se manca, con un percorso solo ASCII: Sunshine non
/// apre la propria configurazione se il percorso ha lettere accentate (es. `C:\Users\Niccolò`)
/// e resta appeso senza scrivere il log.
fn engine_dir(dir: &Path) -> PathBuf {
    let _ = std::fs::create_dir_all(dir);
    ascii_path(dir).unwrap_or_else(|| {
        tracing::warn!("percorso del motore non ASCII e senza nome corto: {}", dir.display());
        dir.to_path_buf()
    })
}

/// Il percorso stesso se è ASCII, altrimenti il nome corto 8.3 di Windows (`NICCOL~1`).
fn ascii_path(dir: &Path) -> Option<PathBuf> {
    if dir.to_str().is_some_and(|s| s.is_ascii()) {
        return Some(dir.to_path_buf());
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        // `for %I in ("…") do echo %~sI` stampa il nome corto senza codice nativo
        let out = std::process::Command::new("cmd")
            .args(["/d", "/c", "for", "%I", "in"])
            .raw_arg(format!("(\"{}\")", dir.display()))
            .args(["do", "@echo", "%~sI"])
            .creation_flags(0x0800_0000)
            .output()
            .ok()?;
        let short = String::from_utf8(out.stdout).ok()?.trim().to_owned();
        if short.is_ascii() && !short.is_empty() && Path::new(&short).is_dir() {
            return Some(PathBuf::from(short));
        }
    }
    None
}

fn encoder_pref(e: Encoder) -> &'static str {
    match e {
        Encoder::Auto => "auto",
        Encoder::Amf => "amf",
        Encoder::Nvenc => "nvenc",
        Encoder::QuickSync => "quicksync",
        Encoder::VideoToolbox => "videotoolbox",
        Encoder::Software => "software",
    }
}

fn device_name() -> String {
    std::env::var("COMPUTERNAME")
        .or_else(|_| std::env::var("HOSTNAME"))
        .unwrap_or_else(|_| "falco-host".into())
        .to_lowercase()
}

impl Host {
    pub fn new(data_dir: &Path) -> Self {
        Self {
            state: HostState::Stopped,
            child: None,
            api: None,
            gpu: None,
            dir: engine_dir(&data_dir.join("sunshine")),
            inputs: sunshine::Inputs::default(),
        }
    }

    /// API solo quando Sunshine è pronto.
    pub fn api(&self) -> Option<Arc<SunshineApi>> {
        self.api.clone().filter(|_| self.state == HostState::Ready)
    }

    /// API anche durante l'avvio, per attendere che sia pronto.
    pub fn api_any(&self) -> Option<Arc<SunshineApi>> {
        self.api.clone()
    }

    pub fn settings(&self, cfg: &Config, inputs: sunshine::Inputs) -> HostSettings {
        let profile = serde_json::to_value(cfg.host.profile)
            .ok()
            .and_then(|v| v.as_str().map(str::to_owned))
            .unwrap_or_default();
        let (fec, cap, quality) = match sunshine::profile_params(&profile) {
            Some((f, c, q)) => (f, c, q.to_owned()),
            None => (
                cfg.network.fec_percent,
                cfg.host.max_bitrate_kbps,
                serde_json::to_value(cfg.host.encoder_quality)
                    .ok()
                    .and_then(|v| v.as_str().map(str::to_owned))
                    .unwrap_or_else(|| "speed".into()),
            ),
        };
        HostSettings {
            max_bitrate_kbps: cap,
            encoder_quality: quality,
            audio_onscreen: cfg.host.audio_capture == falco_store::config::AudioCapture::Onscreen,
            inputs,
            encoder: encoder_pref(cfg.host.encoder).into(),
            max_guests: cfg.host.max_guests,
            upnp: cfg.network.upnp || cfg.network.provider == falco_store::config::Provider::Direct,
            fec_percent: fec,
            base_port: cfg.network.sunshine_base_port,
            // Nome distinto da un eventuale Sunshine già presente sullo stesso PC
            name: format!("{} (Falco)", device_name()),
            dir: self.dir.clone(),
            overrides: {
                let mut o: std::collections::BTreeMap<String, String> = cfg
                    .advanced
                    .sunshine_overrides
                    .iter()
                    .map(|(k, v)| {
                        (k.clone(), v.as_str().map_or_else(|| v.to_string(), str::to_owned))
                    })
                    .collect();
                // Fuori dalla tailnet la cifratura di Sunshine è obbligatoria (§9, §11)
                if cfg.network.provider == falco_store::config::Provider::Direct {
                    o.insert("wan_encryption_mode".into(), "2".into());
                    o.insert("lan_encryption_mode".into(), "1".into());
                }
                o
            },
        }
    }

    /// Scrive la configurazione, imposta le credenziali e avvia Sunshine.
    pub async fn start(&mut self, cfg: &Config, inputs: sunshine::Inputs) -> anyhow::Result<()> {
        if self.child.is_some() {
            return Ok(());
        }
        let bin = falco_engines::locate::sunshine().context("motore host non trovato")?;
        let s = self.settings(cfg, inputs);
        self.inputs = inputs;
        if TcpListener::bind(("0.0.0.0", s.base_port)).is_err() {
            bail!("la porta {} è già usata da un altro programma", s.base_port);
        }
        let gpu = *self.gpu.get_or_insert_with(sunshine::detect_gpu);
        std::fs::create_dir_all(&self.dir)?;
        std::fs::write(s.conf_path(), sunshine::generate_conf(&s, gpu, Platform::current()))?;
        if !s.apps_path().exists() {
            let apps = sunshine::generate_apps(&[]);
            std::fs::write(s.apps_path(), serde_json::to_vec_pretty(&apps)?)?;
        }

        let password = crate::secrets::get_or_create("sunshine-web")?;
        let conf = s.conf_path().to_string_lossy().into_owned();
        let creds = [conf.clone(), "--creds".into(), API_USER.into(), password.clone()];
        // Sunshine esce con 0 anche quando non applica la configurazione: conta il file scritto
        let cred_file = self.dir.join("credentials.json");
        let run = command(&bin, &creds, Priority::Normal).output();
        let out = tokio::time::timeout(std::time::Duration::from_secs(20), run).await;
        if !cred_file.exists() {
            let text = match &out {
                Ok(Ok(o)) => {
                    String::from_utf8_lossy(&o.stdout).into_owned()
                        + &String::from_utf8_lossy(&o.stderr)
                }
                _ => "nessuna risposta in 20 s".into(),
            };
            let _ = std::fs::write(self.dir.join("console.log"), &text);
            let last = text
                .lines()
                .rev()
                .find(|l| l.contains("Failed") || l.contains("rror"))
                .or_else(|| text.lines().rev().find(|l| !l.trim().is_empty()))
                .map(|l| {
                    l.split_once("]   ")
                        .map_or(l, |(_, m)| m)
                        .trim()
                        .chars()
                        .take(200)
                        .collect::<String>()
                })
                .unwrap_or_default();
            bail!("credenziali del motore non impostate: {last}");
        }

        // L'output di console resta su file: gli errori di avvio arrivano prima del log di Sunshine
        let console = std::fs::File::create(self.dir.join("console.log"))?;
        let child = command(&bin, &[conf], Priority::High)
            .stdout(console.try_clone()?)
            .stderr(console)
            .spawn()
            .context("avvio del motore host")?;
        self.child = Some(child);
        self.api = Some(Arc::new(SunshineApi::new(s.web_port(), API_USER, &password)));
        self.state = HostState::Starting;
        Ok(())
    }

    /// L'API risponde? La prima volta Sunshine prova gli encoder (circa 20 s).
    pub async fn api_ready(api: Arc<SunshineApi>) -> bool {
        tokio::task::spawn_blocking(move || api.ready()).await.unwrap_or(false)
    }

    /// Motivo di un avvio fallito: codice d'uscita, poi l'ultimo errore della console o del log.
    pub fn failure_reason(&mut self) -> Option<String> {
        let code = self.child.as_mut().and_then(|c| c.try_wait().ok().flatten()).map(|s| {
            s.code().map_or_else(|| s.to_string(), |c| format!("uscito con codice 0x{c:08X}"))
        });
        let console_tail = || {
            let t = std::fs::read_to_string(self.dir.join("console.log")).ok()?;
            t.lines()
                .rev()
                .find(|l| !l.trim().is_empty())
                .map(|l| l.trim().chars().take(200).collect())
        };
        let text = self
            .last_error_in("console.log")
            .or_else(|| self.last_error_in("sunshine.log"))
            .or_else(console_tail);
        match (code, text) {
            (Some(c), Some(t)) => Some(format!("{t}; {c}")),
            (c, t) => t.or(c),
        }
    }

    /// Ultima riga «Fatal» o «Error» di un log di Sunshine.
    fn last_error_in(&self, file: &str) -> Option<String> {
        let log = std::fs::read_to_string(self.dir.join(file)).ok()?;
        let line = log.lines().rev().find(|l| l.contains("Fatal:")).or_else(|| {
            log.lines().rev().find(|l| l.contains("Error:") && !l.contains("Could not open codec"))
        })?;
        // «[data ora]: Error: messaggio» → «messaggio»
        let msg = line.split_once("]: ").map_or(line, |(_, m)| m);
        Some(msg.split_once(": ").map_or(msg, |(_, m)| m).trim().chars().take(200).collect())
    }

    /// Il processo è ancora vivo?
    pub fn alive(&mut self) -> bool {
        self.child.as_mut().is_some_and(|c| matches!(c.try_wait(), Ok(None)))
    }

    /// Da «Non disponibile» il processo viene fermato, non messo in pausa.
    pub async fn stop(&mut self) {
        if let Some(mut c) = self.child.take() {
            let _ = c.kill().await;
        }
        self.api = None;
        self.state = HostState::Stopped;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(windows)]
    #[test]
    fn accented_dir_gets_ascii_path() {
        let base = tempfile::tempdir().unwrap();
        let dir = base.path().join("Niccolò Falco");
        let p = engine_dir(&dir);
        assert!(p.to_str().unwrap().is_ascii(), "{}", p.display());
        std::fs::write(p.join("x.txt"), "ok").unwrap();
        assert_eq!(std::fs::read_to_string(dir.join("x.txt")).unwrap(), "ok");
    }
}
