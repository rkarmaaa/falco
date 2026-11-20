//! Aggiornamenti (ADR 0008): manifest nel formato dell'updater di Tauri, firma minisign,
//! installazione silenziosa senza procedure manuali.

use std::time::Duration;

use anyhow::{Context, bail};
use data_encoding::BASE64;
use falco_proto::ipc::events;
use falco_store::config::Updates;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::core::Shared;

/// Manifest pubblicato con ogni rilascio (GitHub Releases).
pub const MANIFEST_URL: &str =
    "https://github.com/rkarmaaa/falco/releases/latest/download/latest.json";
/// Chiave pubblica minisign degli aggiornamenti (base64 del file `.pub`).
/// Vuota finché non viene generata la chiave di rilascio: aggiornamenti spenti.
pub const PUBLIC_KEY: &str = "dW50cnVzdGVkIGNvbW1lbnQ6IG1pbmlzaWduIHB1YmxpYyBrZXk6IDM1MkM1QTE3Q0NCMDdERUUKUldUdWZiRE1GMW9zTlZGbXMyWnZKbmUrS1p6V1RWU2s2ZDRiVWFtdEk0ZUFtL1RmNzJScnF2dFIK";

#[derive(Debug, Clone, Deserialize)]
pub struct Manifest {
    pub version: String,
    #[serde(default)]
    pub notes: String,
    pub platforms: std::collections::HashMap<String, PlatformAsset>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PlatformAsset {
    pub signature: String,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Available {
    pub version: String,
    pub notes: String,
}

fn platform_key() -> &'static str {
    if cfg!(all(windows, target_arch = "x86_64")) {
        "windows-x86_64"
    } else if cfg!(all(target_os = "macos", target_arch = "aarch64")) {
        "darwin-aarch64"
    } else if cfg!(target_os = "macos") {
        "darwin-x86_64"
    } else {
        "unsupported"
    }
}

/// Versione più recente del manifest rispetto a quella installata?
pub fn newer(manifest: &Manifest, current: &str) -> bool {
    let parse = |v: &str| semver::Version::parse(v.trim_start_matches('v')).ok();
    matches!((parse(&manifest.version), parse(current)), (Some(m), Some(c)) if m > c)
}

/// Verifica la firma minisign nel formato di Tauri (firma e chiave codificate in base64).
pub fn verify(bytes: &[u8], signature_b64: &str, public_key_b64: &str) -> anyhow::Result<()> {
    let decode = |s: &str| -> anyhow::Result<String> {
        Ok(String::from_utf8(BASE64.decode(s.trim().as_bytes()).context("base64 non valido")?)?)
    };
    let pk = minisign_verify::PublicKey::decode(&decode(public_key_b64)?)
        .context("chiave non valida")?;
    let sig =
        minisign_verify::Signature::decode(&decode(signature_b64)?).context("firma non valida")?;
    pk.verify(bytes, &sig, false).context("firma dell'aggiornamento non valida")?;
    Ok(())
}

fn fetch_manifest() -> anyhow::Result<Manifest> {
    let agent = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(15)))
        .build()
        .new_agent();
    Ok(agent.get(MANIFEST_URL).call()?.body_mut().read_json()?)
}

/// Controlla la presenza di un aggiornamento e lo segnala (evento, tray, notifica).
pub async fn check(shared: &Shared) -> anyhow::Result<Option<Available>> {
    if PUBLIC_KEY.is_empty() {
        return Ok(None);
    }
    let m = tokio::task::spawn_blocking(fetch_manifest).await??;
    if !newer(&m, env!("CARGO_PKG_VERSION")) || !m.platforms.contains_key(platform_key()) {
        return Ok(None);
    }
    let a = Available { version: m.version.clone(), notes: m.notes.clone() };
    let mut c = shared.lock().await;
    if c.update.as_ref() != Some(&a) {
        c.emit(events::UPDATE_AVAILABLE, json!(a));
        c.notify_user(
            &format!("È disponibile Falco {}", a.version),
            "Apri Falco per aggiornare con un clic.",
        );
        c.update = Some(a.clone());
        c.refresh_tray();
    }
    Ok(Some(a))
}

/// Scarica, verifica e avvia l'installer silenzioso; poi il core si chiude.
pub async fn install(shared: &Shared) -> anyhow::Result<()> {
    if shared.lock().await.streaming {
        bail!("c'è una sessione in corso: l'aggiornamento partirà alla fine");
    }
    let dir = shared.lock().await.data_dir.join("updates");
    std::fs::create_dir_all(&dir)?;
    let m = tokio::task::spawn_blocking(fetch_manifest).await??;
    let asset =
        m.platforms.get(platform_key()).context("nessun pacchetto per questo sistema")?.clone();
    let bytes = tokio::task::spawn_blocking(move || -> anyhow::Result<Vec<u8>> {
        Ok(ureq::get(&asset.url).call()?.body_mut().with_config().limit(512 << 20).read_to_vec()?)
    })
    .await??;
    let sig = m.platforms[platform_key()].signature.clone();
    verify(&bytes, &sig, PUBLIC_KEY)?;
    let file = dir.join(if cfg!(windows) { "falco-setup.exe" } else { "falco.app.tar.gz" });
    std::fs::write(&file, &bytes)?;
    {
        let c = shared.lock().await;
        // Finestra aperta: il core nuovo la riapre appena parte (lo avvia l'installer, nel tray)
        if c.ui_clients > 0 {
            let _ = std::fs::write(reopen_marker(&c.data_dir), b"");
        }
        c.emit(events::UPDATE_PROGRESS, json!({ "step": "installing" }));
    }
    if cfg!(windows) {
        // NSIS silenzioso: l'installer chiude e riavvia Falco. Il core gira come utente normale e
        // l'installer è per tutto il PC: si avvia con «runas», Windows chiede la conferma (UAC)
        let path = file.to_string_lossy().replace('\'', "''");
        let script = format!("Start-Process -FilePath '{path}' -ArgumentList '/S' -Verb RunAs");
        let mut cmd = std::process::Command::new("powershell");
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
        }
        let status = cmd
            .args(["-NoProfile", "-NonInteractive", "-WindowStyle", "Hidden", "-Command", &script])
            .status()
            .context("installer non avviato")?;
        if !status.success() {
            bail!("aggiornamento annullato: serve confermare la richiesta di Windows");
        }
    } else {
        let app = std::env::current_exe()?
            .ancestors()
            .nth(3)
            .context("bundle non trovato")?
            .to_path_buf();
        // Estrae a parte e poi sostituisce il pacchetto intero: niente file vecchi che
        // invaliderebbero la firma. Si riapre dopo la chiusura di questa istanza (istanza singola)
        let parent = app.parent().context("cartella delle applicazioni")?;
        let staging = parent.join(".falco-update");
        let _ = std::fs::remove_dir_all(&staging);
        std::fs::create_dir_all(&staging)?;
        let status = std::process::Command::new("tar")
            .arg("-xzf")
            .arg(&file)
            .arg("-C")
            .arg(&staging)
            .status()?;
        anyhow::ensure!(status.success(), "aggiornamento non estratto");
        let new_app = std::fs::read_dir(&staging)?
            .filter_map(Result::ok)
            .map(|e| e.path())
            .find(|p| p.extension().is_some_and(|x| x == "app"))
            .context("pacchetto dell'aggiornamento vuoto")?;
        let old = app.with_extension("app.old");
        let _ = std::fs::remove_dir_all(&old);
        std::fs::rename(&app, &old).context("Falco non è aggiornabile in questa cartella")?;
        if let Err(e) = std::fs::rename(&new_app, &app) {
            let _ = std::fs::rename(&old, &app);
            return Err(e).context("aggiornamento non installato");
        }
        let _ = std::fs::remove_dir_all(&old);
        let _ = std::fs::remove_dir_all(&staging);
        std::process::Command::new("sh").arg("-c").arg("sleep 2; open \"$0\"").arg(&app).spawn()?;
    }
    shared.lock().await.shutdown().await;
    std::process::exit(0);
}

/// All'avvio e ogni 6 ore; in modalità automatica installa quando non si gioca.
/// File lasciato prima di aggiornare se la finestra era aperta: il core nuovo la riapre.
fn reopen_marker(data_dir: &std::path::Path) -> std::path::PathBuf {
    data_dir.join("updates").join("reopen-ui")
}

/// Il core è appena stato aggiornato con la finestra aperta? Consuma il segnale.
pub fn take_reopen(data_dir: &std::path::Path) -> bool {
    std::fs::remove_file(reopen_marker(data_dir)).is_ok()
}

/// Falco è in uso? Un aggiornamento chiude core e motori: mai durante uno stream, una
/// condivisione, un pairing, un ingresso in corso o con la finestra aperta (si riprova dopo).
fn busy(c: &crate::core::Core) -> bool {
    c.streaming
        || !c.guests.is_empty()
        || c.host_available
        || c.joined.is_some()
        || !c.knocks.is_empty()
        || c.ui_clients > 0
        || crate::invites::pairing_active()
}

pub async fn run(shared: Shared) {
    tokio::time::sleep(Duration::from_secs(20)).await;
    loop {
        let mode = shared.lock().await.config.config().general.updates;
        let mut pending = false;
        if mode != Updates::Off {
            match check(&shared).await {
                Ok(Some(_)) if mode == Updates::Auto => {
                    if busy(&*shared.lock().await) {
                        pending = true;
                    } else if let Err(e) = install(&shared).await {
                        shared.lock().await.record_error("update.install", &format!("{e:#}"), None);
                    }
                }
                Ok(_) => {}
                Err(e) => tracing::info!("controllo aggiornamenti: {e:#}"),
            }
        }
        // In attesa che Falco non sia in uso: si riprova presto, altrimenti ogni 6 ore
        let wait = if pending { 10 * 60 } else { 6 * 3600 };
        tokio::time::sleep(Duration::from_secs(wait)).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn manifest(v: &str) -> Manifest {
        Manifest { version: v.into(), notes: String::new(), platforms: Default::default() }
    }

    #[test]
    fn version_comparison() {
        assert!(newer(&manifest("1.0.1"), "1.0.0"));
        assert!(newer(&manifest("v1.1.0"), "1.0.0-rc.1"));
        assert!(!newer(&manifest("1.0.0"), "1.0.0"));
        assert!(!newer(&manifest("0.9.0"), "1.0.0"));
        assert!(!newer(&manifest("non-una-versione"), "1.0.0"));
    }

    #[test]
    fn real_release_key_verifies_signed_sample() {
        let bin = include_bytes!("../tests/fixtures/update-sample.bin");
        let sig = include_str!("../tests/fixtures/update-sample.bin.sig");
        verify(bin, sig, PUBLIC_KEY).unwrap();
        // Un solo byte cambiato e la firma non vale più
        let mut tampered = bin.to_vec();
        tampered[0] ^= 1;
        assert!(verify(&tampered, sig, PUBLIC_KEY).is_err());
    }

    #[test]
    fn rejects_bad_signatures() {
        assert!(verify(b"x", "bm9u", "bm9u").is_err());
    }
}
