//! Host: un'istanza di Sunshine dedicata a Falco, viva solo quando si è «Disponibile» (§8.1).

use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

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
            dir: data_dir.join("sunshine"),
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

    pub fn settings(&self, cfg: &Config) -> HostSettings {
        HostSettings {
            encoder: encoder_pref(cfg.host.encoder).into(),
            max_guests: cfg.host.max_guests,
            upnp: cfg.network.upnp || cfg.network.provider == falco_store::config::Provider::Direct,
            fec_percent: cfg.network.fec_percent,
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
    pub async fn start(&mut self, cfg: &Config) -> anyhow::Result<()> {
        if self.child.is_some() {
            return Ok(());
        }
        let bin = falco_engines::locate::sunshine().context("motore host non trovato")?;
        let s = self.settings(cfg);
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
        let status = command(&bin, &creds, Priority::Normal).status().await?;
        if !status.success() {
            bail!("credenziali del motore non impostate");
        }

        let child =
            command(&bin, &[conf], Priority::High).spawn().context("avvio del motore host")?;
        self.child = Some(child);
        self.api = Some(Arc::new(SunshineApi::new(s.web_port(), API_USER, &password)));
        self.state = HostState::Starting;
        Ok(())
    }

    /// Attende che l'API risponda: la prima volta Sunshine prova gli encoder (circa 20 s).
    pub async fn wait_ready(api: Arc<SunshineApi>) -> bool {
        for _ in 0..90 {
            let a = api.clone();
            if tokio::task::spawn_blocking(move || a.ready()).await.unwrap_or(false) {
                return true;
            }
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
        false
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
