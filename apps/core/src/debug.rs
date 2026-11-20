//! Diagnostica per gli errori: registro degli errori recenti e report dettagliato copiabile.
//! Mai segreti nel report: niente parametri delle chiamate (possono contenere link di invito).

use std::collections::VecDeque;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

use serde::Serialize;
use serde_json::{Value, json};

use crate::core::{Core, Shared, now};

const MAX_ERRORS: usize = 50;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ErrorEntry {
    pub at: i64,
    /// Origine: metodo IPC (`session.join`) o attività di sfondo (`host`, `invite`, …).
    pub source: String,
    pub code: Option<i32>,
    pub message: String,
    pub data: Option<Value>,
}

#[derive(Debug, Default)]
pub struct ErrorLog {
    entries: VecDeque<ErrorEntry>,
}

impl ErrorLog {
    pub fn push(&mut self, source: &str, code: Option<i32>, message: &str, data: Option<Value>) {
        if self.entries.len() == MAX_ERRORS {
            self.entries.pop_front();
        }
        self.entries.push_back(ErrorEntry {
            at: now(),
            source: source.to_owned(),
            code,
            message: message.to_owned(),
            data,
        });
    }

    pub fn recent(&self) -> Vec<ErrorEntry> {
        self.entries.iter().rev().cloned().collect()
    }
}

/// Ultime righe di un file di testo, leggendo solo la coda (i log possono essere grandi).
pub fn tail(path: &Path, lines: usize) -> Vec<String> {
    let Ok(mut f) = std::fs::File::open(path) else { return Vec::new() };
    let len = f.metadata().map(|m| m.len()).unwrap_or(0);
    let start = len.saturating_sub(64 * 1024);
    if f.seek(SeekFrom::Start(start)).is_err() {
        return Vec::new();
    }
    let mut buf = Vec::new();
    let _ = f.read_to_end(&mut buf);
    let text = String::from_utf8_lossy(&buf);
    let all: Vec<&str> = text.lines().collect();
    all[all.len().saturating_sub(lines)..].iter().map(|s| (*s).to_owned()).collect()
}

/// Log del core più recente nella cartella dei log.
fn latest_log(dir: &Path) -> Option<std::path::PathBuf> {
    std::fs::read_dir(dir)
        .ok()?
        .filter_map(Result::ok)
        .filter(|e| e.file_name().to_string_lossy().starts_with("core"))
        .max_by_key(|e| e.metadata().and_then(|m| m.modified()).ok())
        .map(|e| e.path())
}

fn engine(path: Option<std::path::PathBuf>) -> Value {
    match path {
        Some(p) => json!({ "path": p.to_string_lossy(), "exists": p.is_file() }),
        None => json!({ "path": null, "exists": false }),
    }
}

fn snapshot(c: &Core, net: &falco_net::NetStatus) -> Value {
    let cfg = c.config.config();
    json!({
        "app": {
            "version": env!("CARGO_PKG_VERSION"),
            "milestone": crate::core::MILESTONE,
            "os": std::env::consts::OS,
            "arch": std::env::consts::ARCH,
            "instance": falco_proto::endpoint::instance(),
            "exe": std::env::current_exe().ok().map(|p| p.to_string_lossy().into_owned()),
            "time": now(),
        },
        "state": {
            "hasProfile": c.me.is_some(),
            "status": c.status(),
            "hostAvailable": c.host_available,
            "host": c.host.state,
            "streaming": c.streaming,
            "guests": c.guests.len(),
            "friends": c.db.friend_count().unwrap_or(0),
            "uiClients": c.ui_clients,
            "updateAvailable": c.update.as_ref().map(|u| u.version.clone()),
        },
        "network": {
            "provider": cfg.network.provider,
            "tailscale": net.state,
            "selfIps": net.self_ips,
            "selfNode": net.self_node_id,
            "peers": net.peers.len(),
            "exitNodeActive": net.exit_node_active,
            "lanPeers": c.lan_peers.len(),
            "presencePort": cfg.network.presence_port,
            "sunshinePort": cfg.network.sunshine_base_port,
        },
        "engines": {
            "sunshine": engine(falco_engines::locate::sunshine()),
            "moonlight": engine(falco_engines::locate::moonlight()),
            "tailscaleCli": falco_net::tailscale::locate_cli().map(|p| p.to_string_lossy().into_owned()),
        },
        "config": cfg,
        "lastSession": c.db.meta("last_session").ok().flatten().and_then(|s| serde_json::from_str::<Value>(&s).ok()),
        "errors": c.errors.recent(),
    })
}

/// Report completo per la schermata «Debug» della UI.
pub async fn report(shared: &Shared, logs_dir: &Path) -> Value {
    let ts = shared.lock().await.net.clone();
    let net = crate::core::Core::net_status(ts).await;
    let c = shared.lock().await;
    let mut v = snapshot(&c, &net);
    let sunshine_log = c.data_dir.join("sunshine").join("sunshine.log");
    let moonlight_log = c.data_dir.join("moonlight").join("stream.log");
    drop(c);
    v["logs"] = json!({
        "core": latest_log(logs_dir).map(|p| tail(&p, 200)).unwrap_or_default(),
        "sunshine": tail(&sunshine_log, 60),
        "moonlight": tail(&moonlight_log, 80),
    });
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ring_buffer_keeps_latest() {
        let mut log = ErrorLog::default();
        for i in 0..60 {
            log.push("test", Some(i), &format!("errore {i}"), None);
        }
        let r = log.recent();
        assert_eq!(r.len(), MAX_ERRORS);
        assert_eq!(r[0].message, "errore 59");
    }

    #[test]
    fn tail_reads_last_lines() {
        let dir = std::env::temp_dir().join(format!("falco-tail-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let p = dir.join("x.log");
        std::fs::write(&p, (0..500).map(|i| format!("riga {i}\n")).collect::<String>()).unwrap();
        let t = tail(&p, 3);
        assert_eq!(t, ["riga 497", "riga 498", "riga 499"]);
        let _ = std::fs::remove_dir_all(dir);
    }
}
