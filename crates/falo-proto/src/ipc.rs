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
}

pub mod events {
    pub const PRESENCE_CHANGED: &str = "presence.changed";
    pub const CONFIG_CHANGED: &str = "config.changed";
    pub const INVITE_PROGRESS: &str = "invite.progress";
    pub const SESSION_KNOCK: &str = "session.knock";
    pub const SESSION_STATS: &str = "session.stats";
    pub const SESSION_ENDED: &str = "session.ended";
    pub const NET_WARNING: &str = "net.warning";
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
    pub milestone: String,
    pub status: Status,
    pub host_available: bool,
    pub streaming: bool,
    pub network: NetworkSummary,
    pub friends_count: usize,
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
