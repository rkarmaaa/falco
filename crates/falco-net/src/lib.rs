//! Rete di Falco (§9). Tutta la logica Tailscale vive qui, dietro `NetworkProvider`.
#![cfg_attr(test, allow(clippy::unwrap_used))]

pub mod diagnostics;
pub mod doctor;
pub mod lan;
pub mod tailscale;
pub mod tailscale_api;

use std::future::Future;
use std::net::IpAddr;

pub use falco_proto::ipc::TailscaleState;
use serde::Serialize;

#[derive(Debug, thiserror::Error)]
pub enum NetError {
    #[error("Tailscale non è installato")]
    TailscaleMissing,
    #[error("Tailscale non ha risposto in tempo")]
    Timeout,
    #[error("risposta di Tailscale non valida: {0}")]
    BadOutput(String),
    #[error("errore di sistema: {0}")]
    Io(#[from] std::io::Error),
}

/// Percorso verso un peer: diretto, via relay DERP o sconosciuto (nessun traffico recente).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase", tag = "kind")]
pub enum PeerPath {
    Direct { endpoint: String },
    Relay { region: String },
    Idle,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Peer {
    pub node_id: String,
    pub hostname: String,
    pub os: String,
    pub ips: Vec<IpAddr>,
    pub online: bool,
    pub path: PeerPath,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NetStatus {
    pub state: TailscaleState,
    pub self_node_id: Option<String>,
    pub self_ips: Vec<IpAddr>,
    pub peers: Vec<Peer>,
    pub exit_node_active: bool,
}

impl NetStatus {
    pub fn missing() -> Self {
        Self {
            state: TailscaleState::Missing,
            self_node_id: None,
            self_ips: Vec::new(),
            peers: Vec::new(),
            exit_node_active: false,
        }
    }
}

/// Interfaccia comune ai provider Tailscale, LAN, Diretto e Headscale.
pub trait NetworkProvider {
    fn kind(&self) -> &'static str;
    /// Stato della rete e dei peer visibili.
    fn status(&self) -> impl Future<Output = Result<NetStatus, NetError>> + Send;
    /// Indirizzo da usare per raggiungere un nodo.
    fn resolve(
        &self,
        node_id: &str,
    ) -> impl Future<Output = Result<Option<IpAddr>, NetError>> + Send;
}
