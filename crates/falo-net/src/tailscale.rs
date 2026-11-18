//! Provider Tailscale: pilota la CLI ufficiale con `--json` (§3, §9).

use std::collections::HashMap;
use std::net::IpAddr;
use std::path::PathBuf;
use std::time::Duration;

use serde::Deserialize;
use tokio::process::Command;

use crate::{NetError, NetStatus, NetworkProvider, Peer, PeerPath, TailscaleState};

const CLI_TIMEOUT: Duration = Duration::from_secs(5);

/// Cerca la CLI: `FALO_TAILSCALE_BIN` (test), PATH, poi i percorsi d'installazione.
pub fn locate_cli() -> Option<PathBuf> {
    if let Some(p) = std::env::var_os("FALO_TAILSCALE_BIN") {
        return Some(PathBuf::from(p));
    }
    if let Ok(p) = which::which("tailscale") {
        return Some(p);
    }
    let candidates: &[&str] = if cfg!(windows) {
        &[r"C:\Program Files\Tailscale\tailscale.exe"]
    } else {
        &[
            "/Applications/Tailscale.app/Contents/MacOS/Tailscale",
            "/usr/local/bin/tailscale",
            "/opt/homebrew/bin/tailscale",
        ]
    };
    candidates.iter().map(PathBuf::from).find(|p| p.exists())
}

#[derive(Debug, Clone)]
pub struct Tailscale {
    bin: Option<PathBuf>,
}

impl Tailscale {
    pub fn detect() -> Self {
        Self { bin: locate_cli() }
    }

    pub fn with_bin(bin: impl Into<PathBuf>) -> Self {
        Self { bin: Some(bin.into()) }
    }

    pub fn installed(&self) -> bool {
        self.bin.is_some()
    }

    /// Esegue la CLI con timeout, senza finestra di console su Windows.
    pub async fn run(&self, args: &[&str]) -> Result<Vec<u8>, NetError> {
        let bin = self.bin.as_ref().ok_or(NetError::TailscaleMissing)?;
        let mut cmd = Command::new(bin);
        cmd.args(args).kill_on_drop(true);
        #[cfg(windows)]
        cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
        let out = tokio::time::timeout(CLI_TIMEOUT, cmd.output())
            .await
            .map_err(|_| NetError::Timeout)??;
        // `status` esce con codice ≠ 0 quando è fermo ma stampa comunque il JSON
        Ok(out.stdout)
    }
}

#[derive(Deserialize)]
struct Whois {
    #[serde(rename = "Node")]
    node: WhoisNode,
}

#[derive(Deserialize)]
struct WhoisNode {
    #[serde(rename = "StableID")]
    stable_id: String,
}

impl Tailscale {
    /// Nodo Tailscale (ID stabile) che possiede un indirizzo: doppio vincolo del §5.
    pub async fn whois(&self, ip: IpAddr) -> Result<Option<String>, NetError> {
        let out = self.run(&["whois", "--json", &ip.to_string()]).await?;
        Ok(parse_whois(&out))
    }
}

pub fn parse_whois(json: &[u8]) -> Option<String> {
    serde_json::from_slice::<Whois>(json).ok().map(|w| w.node.stable_id)
}

impl NetworkProvider for Tailscale {
    fn kind(&self) -> &'static str {
        "tailscale"
    }

    async fn status(&self) -> Result<NetStatus, NetError> {
        if !self.installed() {
            return Ok(NetStatus::missing());
        }
        let out = self.run(&["status", "--json"]).await?;
        parse_status(&out)
    }

    async fn resolve(&self, node_id: &str) -> Result<Option<IpAddr>, NetError> {
        let status = self.status().await?;
        Ok(status
            .peers
            .into_iter()
            .find(|p| p.node_id == node_id)
            .and_then(|p| p.ips.into_iter().next()))
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct RawStatus {
    backend_state: String,
    #[serde(rename = "Self")]
    self_node: Option<RawPeer>,
    #[serde(default)]
    peer: Option<HashMap<String, RawPeer>>,
    #[serde(rename = "TailscaleIPs", default)]
    tailscale_ips: Option<Vec<IpAddr>>,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct RawPeer {
    #[serde(rename = "ID", default)]
    id: String,
    #[serde(default)]
    host_name: String,
    #[serde(rename = "OS", default)]
    os: String,
    #[serde(rename = "TailscaleIPs", default)]
    tailscale_ips: Option<Vec<IpAddr>>,
    #[serde(default)]
    online: bool,
    #[serde(default)]
    exit_node: bool,
    #[serde(default)]
    relay: String,
    #[serde(default)]
    cur_addr: String,
}

/// Converte l'output di `tailscale status --json`.
pub fn parse_status(json: &[u8]) -> Result<NetStatus, NetError> {
    let raw: RawStatus =
        serde_json::from_slice(json).map_err(|e| NetError::BadOutput(e.to_string()))?;
    let state = match raw.backend_state.as_str() {
        "Running" => TailscaleState::Running,
        "NeedsLogin" | "NeedsMachineAuth" | "NoState" => TailscaleState::NeedsLogin,
        _ => TailscaleState::Stopped,
    };
    let exit_node_active = raw.peer.as_ref().is_some_and(|m| m.values().any(|p| p.exit_node));
    let mut peers: Vec<Peer> = raw
        .peer
        .unwrap_or_default()
        .into_values()
        .map(|p| Peer {
            path: if !p.cur_addr.is_empty() {
                PeerPath::Direct { endpoint: p.cur_addr }
            } else if !p.relay.is_empty() && p.online {
                PeerPath::Relay { region: p.relay }
            } else {
                PeerPath::Idle
            },
            node_id: p.id,
            hostname: p.host_name,
            os: p.os,
            ips: p.tailscale_ips.unwrap_or_default(),
            online: p.online,
        })
        .collect();
    peers.sort_by(|a, b| a.hostname.cmp(&b.hostname));
    Ok(NetStatus {
        state,
        self_node_id: raw.self_node.as_ref().map(|s| s.id.clone()),
        self_ips: raw.tailscale_ips.unwrap_or_default(),
        peers,
        exit_node_active,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &str = include_str!("../tests/fixtures/status_running.json");

    #[test]
    fn parses_running_status() {
        let s = parse_status(FIXTURE.as_bytes()).unwrap();
        assert_eq!(s.state, TailscaleState::Running);
        assert_eq!(s.self_ips[0].to_string(), "100.101.102.103");
        assert_eq!(s.peers.len(), 2);
        let luca = s.peers.iter().find(|p| p.hostname == "luca-pc").unwrap();
        assert!(matches!(luca.path, PeerPath::Direct { .. }));
        let mac = s.peers.iter().find(|p| p.hostname == "bea-mac").unwrap();
        assert_eq!(mac.path, PeerPath::Relay { region: "fra".into() });
        assert!(!s.exit_node_active);
    }

    #[test]
    fn whois_stable_id() {
        let j =
            br#"{"Node":{"ID":66005,"StableID":"nSTzF88PYt11CNTRL","Name":"x"},"UserProfile":{}}"#;
        assert_eq!(parse_whois(j).as_deref(), Some("nSTzF88PYt11CNTRL"));
        assert_eq!(parse_whois(b"no peer found"), None);
    }

    #[test]
    fn needs_login_and_garbage() {
        let s = parse_status(br#"{"BackendState":"NeedsLogin","Self":null,"Peer":null}"#).unwrap();
        assert_eq!(s.state, TailscaleState::NeedsLogin);
        assert!(parse_status(b"not json").is_err());
    }

    #[tokio::test(flavor = "current_thread")]
    async fn missing_cli_is_reported_as_state() {
        let ts = Tailscale { bin: None };
        assert_eq!(ts.status().await.unwrap().state, TailscaleState::Missing);
    }
}
