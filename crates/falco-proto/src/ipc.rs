//! Contratto IPC core ↔ UI: JSON-RPC 2.0, un messaggio JSON per riga.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::profile::Status;

pub const VERSION: &str = "2.0";

pub mod methods {
    pub const STATE_SNAPSHOT: &str = "state.snapshot";
    pub const CONFIG_GET: &str = "config.get";
    pub const CONFIG_SET: &str = "config.set";
    pub const CONFIG_RESET: &str = "config.reset";
    pub const PROFILE_UPDATE: &str = "profile.update";
    pub const FRIENDS_LIST: &str = "friends.list";
    pub const FRIEND_SET_PERMISSIONS: &str = "friend.setPermissions";
    pub const INVITE_CREATE: &str = "invite.create";
    pub const INVITE_ACCEPT: &str = "invite.accept";
    pub const SESSION_JOIN: &str = "session.join";
    pub const HOST_SET_AVAILABLE: &str = "host.setAvailable";
    pub const NET_RUN_DOCTOR: &str = "net.runDoctor";
    pub const APP_OPEN_UI: &str = "app.openUi";
    pub const HOST_PAIRINGS: &str = "host.pairings";
    pub const HOST_APPROVE: &str = "host.approvePairing";
    pub const HOST_CLIENTS: &str = "host.clients";
    pub const HOST_UNPAIR: &str = "host.unpair";
    pub const HOST_SET_CLIENT_ENABLED: &str = "host.setClientEnabled";
    pub const HOST_END_SESSION: &str = "host.endSession";
    pub const SESSION_PAIR: &str = "session.pair";
    pub const SESSION_LEAVE: &str = "session.leave";
    pub const PROFILE_GET: &str = "profile.get";
    pub const PROFILE_CREATE: &str = "profile.create";
    pub const FRIENDS_REFRESH: &str = "friends.refresh";
    pub const FRIEND_REMOVE: &str = "friend.remove";
    pub const FRIEND_BLOCK: &str = "friend.block";
    pub const FRIEND_FAVORITE: &str = "friend.favorite";
    pub const INVITE_LIST: &str = "invite.list";
    pub const INVITE_REVOKE: &str = "invite.revoke";
    pub const INVITE_PREVIEW: &str = "invite.preview";
    pub const INVITE_APPROVE: &str = "invite.approve";
    pub const TAILSCALE_SET_OAUTH: &str = "tailscale.setOAuth";
    pub const FRIEND_GET_PERMISSIONS: &str = "friend.getPermissions";
    pub const FRIEND_NOTIFY: &str = "friend.notify";
    pub const HOST_ANSWER_KNOCK: &str = "host.answerKnock";
    pub const HOST_KICK: &str = "host.kick";
    pub const HOST_GUESTS: &str = "host.guests";
    pub const CONFIG_SCHEMA: &str = "config.schema";
    pub const CONFIG_EXPORT: &str = "config.export";
    pub const CONFIG_IMPORT: &str = "config.import";
    pub const NET_LAN_PEERS: &str = "net.lanPeers";
    pub const NET_LOGIN_HEADSCALE: &str = "net.loginHeadscale";
    pub const UPDATE_CHECK: &str = "update.check";
    pub const UPDATE_INSTALL: &str = "update.install";
    pub const TAILSCALE_INSTALL: &str = "tailscale.install";
    // Falco Hub (ADR 0017)
    pub const HUB_INFO: &str = "hub.info";
    pub const HUB_REQUESTS: &str = "hub.requests";
    pub const HUB_ADD_FRIEND: &str = "hub.addFriend";
    pub const HUB_ACCEPT: &str = "hub.accept";
    pub const HUB_DECLINE: &str = "hub.decline";
    pub const HUB_SHARE_STREAM: &str = "hub.shareStream";
    pub const HUB_JOIN: &str = "hub.join";
    pub const NET_JOIN_FALCO: &str = "net.joinFalco";
    pub const APP_OPEN_LINK: &str = "app.openLink";
    pub const DEBUG_REPORT: &str = "debug.report";
    pub const HOST_RECOMMEND: &str = "host.recommend";
    pub const HOST_APPLY_INPUTS: &str = "host.applyInputs";
}

pub mod events {
    pub const PRESENCE_CHANGED: &str = "presence.changed";
    pub const CONFIG_CHANGED: &str = "config.changed";
    pub const INVITE_PROGRESS: &str = "invite.progress";
    pub const SESSION_KNOCK: &str = "session.knock";
    pub const SESSION_STATS: &str = "session.stats";
    pub const SESSION_ENDED: &str = "session.ended";
    /// Moonlight avviato: `{ host, name, pid, startedAt, permissions? }` (la UI incorpora la finestra).
    pub const SESSION_STARTED: &str = "session.started";
    /// Input ricevuti dagli ospiti: `{ keyboard, mouse, controller }` (al massimo 4 volte al secondo).
    pub const HOST_INPUT: &str = "host.input";
    pub const NET_WARNING: &str = "net.warning";
    pub const HOST_STATE: &str = "host.state";
    pub const PAIR_RESULT: &str = "pair.result";
    pub const INVITE_REQUEST: &str = "invite.request";
    pub const HOST_GUESTS: &str = "host.guests";
    pub const PLAY_INVITE: &str = "play.invite";
    /// Richieste d'amicizia cambiate sul Hub: `{ incoming, outgoing }`.
    pub const HUB_REQUESTS: &str = "hub.requests";
    pub const UI_NAVIGATE: &str = "ui.navigate";
    pub const UPDATE_AVAILABLE: &str = "update.available";
    pub const UPDATE_PROGRESS: &str = "update.progress";
    pub const UI_OPEN_LINK: &str = "ui.openLink";
    /// Errore di un'attività di sfondo (host, pairing, inviti, sessioni, aggiornamenti).
    pub const ERROR_OCCURRED: &str = "error.occurred";
}

pub mod codes {
    pub const PARSE_ERROR: i32 = -32700;
    pub const INVALID_REQUEST: i32 = -32600;
    pub const METHOD_NOT_FOUND: i32 = -32601;
    pub const INVALID_PARAMS: i32 = -32602;
    pub const INTERNAL: i32 = -32603;
    /// Metodo previsto dalla specifica ma non ancora implementato.
    pub const NOT_IMPLEMENTED: i32 = -32001;
    pub const CONFIG_INVALID: i32 = -32002;
    pub const HOST_NOT_READY: i32 = -32003;
    pub const SESSION_FAILED: i32 = -32004;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Request {
    pub jsonrpc: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<Value>,
    pub method: String,
    #[serde(default)]
    pub params: Value,
}

impl Request {
    pub fn new(id: u64, method: &str, params: Value) -> Self {
        Self { jsonrpc: VERSION.into(), id: Some(id.into()), method: method.into(), params }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RpcError {
    pub code: i32,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

impl RpcError {
    pub fn new(code: i32, message: impl Into<String>) -> Self {
        Self { code, message: message.into(), data: None }
    }

    pub fn with_data(mut self, data: Value) -> Self {
        self.data = Some(data);
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Response {
    pub jsonrpc: String,
    pub id: Value,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<RpcError>,
}

impl Response {
    pub fn ok(id: Value, result: Value) -> Self {
        Self { jsonrpc: VERSION.into(), id, result: Some(result), error: None }
    }

    pub fn err(id: Value, error: RpcError) -> Self {
        Self { jsonrpc: VERSION.into(), id, result: None, error: Some(error) }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Notification {
    pub jsonrpc: String,
    pub method: String,
    #[serde(default)]
    pub params: Value,
}

impl Notification {
    pub fn new(method: &str, params: Value) -> Self {
        Self { jsonrpc: VERSION.into(), method: method.into(), params }
    }
}

/// Qualsiasi messaggio in arrivo dal core.
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum Incoming {
    Response(Response),
    Notification(Notification),
}

/// Serializza un messaggio come riga terminata da `\n`.
pub fn to_line<T: Serialize>(msg: &T) -> Result<Vec<u8>, serde_json::Error> {
    let mut out = serde_json::to_vec(msg)?;
    out.push(b'\n');
    Ok(out)
}

/// Stato della rete come lo vede la UI.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TailscaleState {
    Missing,
    Stopped,
    NeedsLogin,
    Running,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NetworkSummary {
    pub provider: String,
    pub tailscale: TailscaleState,
    pub self_ips: Vec<String>,
    pub exit_node_active: bool,
}

/// Snapshot completo per idratare la UI in un solo messaggio.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StateSnapshot {
    pub version: String,
    /// `{ state: "stopped" | "starting" | "ready" | "error", message? }`
    pub host: serde_json::Value,
    pub milestone: String,
    pub status: Status,
    pub host_available: bool,
    pub streaming: bool,
    pub network: NetworkSummary,
    pub friends_count: usize,
    /// Il profilo esiste? Se no, la UI mostra l'onboarding.
    pub has_profile: bool,
    pub oauth_configured: bool,
    /// Sessione da ospite in corso (stessi campi di `session.started`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session: Option<serde_json::Value>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn incoming_distinguishes_responses_and_events() {
        let r: Incoming =
            serde_json::from_str(r#"{"jsonrpc":"2.0","id":1,"result":true}"#).unwrap();
        assert!(matches!(r, Incoming::Response(_)));
        let n: Incoming =
            serde_json::from_str(r#"{"jsonrpc":"2.0","method":"presence.changed","params":{}}"#)
                .unwrap();
        assert!(matches!(n, Incoming::Notification(_)));
    }

    #[test]
    fn line_framing() {
        let line = to_line(&Request::new(7, methods::STATE_SNAPSHOT, json!(null))).unwrap();
        assert_eq!(line.last(), Some(&b'\n'));
        assert_eq!(line.iter().filter(|b| **b == b'\n').count(), 1);
    }
}
