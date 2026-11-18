//! Protocollo di presenza Falò (FPP v1, §5): tipi e firma delle richieste.

use data_encoding::HEXLOWER;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::identity::{FaloId, Identity};
use crate::profile::Status;

pub const HEADER_ID: &str = "x-falo-id";
pub const HEADER_TS: &str = "x-falo-ts";
pub const HEADER_SIG: &str = "x-falo-sig";
/// Finestra di validità del timestamp, in secondi.
pub const MAX_SKEW_S: i64 = 60;

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum AuthError {
    #[error("intestazioni di autenticazione mancanti")]
    Missing,
    #[error("orario della richiesta fuori finestra")]
    Skew,
    #[error("firma non valida")]
    Signature,
}

/// Messaggio firmato: metodo, percorso, timestamp e SHA-256 del corpo.
fn message(method: &str, path: &str, ts: i64, body: &[u8]) -> Vec<u8> {
    let digest = HEXLOWER.encode(&Sha256::digest(body));
    format!("{}\n{path}\n{ts}\n{digest}", method.to_ascii_uppercase()).into_bytes()
}

/// Intestazioni da aggiungere a una richiesta verso un amico.
pub fn sign_request(
    me: &Identity,
    method: &str,
    path: &str,
    ts: i64,
    body: &[u8],
) -> [(&'static str, String); 3] {
    [
        (HEADER_ID, me.id().to_string()),
        (HEADER_TS, ts.to_string()),
        (HEADER_SIG, me.sign(&message(method, path, ts, body))),
    ]
}

/// Verifica una richiesta in arrivo e restituisce l'ID del mittente.
pub fn verify_request(
    id: Option<&str>,
    ts: Option<&str>,
    sig: Option<&str>,
    method: &str,
    path: &str,
    body: &[u8],
    now: i64,
) -> Result<FaloId, AuthError> {
    let (Some(id), Some(ts), Some(sig)) = (id, ts, sig) else { return Err(AuthError::Missing) };
    let id: FaloId = id.parse().map_err(|_| AuthError::Signature)?;
    let ts: i64 = ts.parse().map_err(|_| AuthError::Missing)?;
    if (now - ts).abs() > MAX_SKEW_S {
        return Err(AuthError::Skew);
    }
    id.verify(&message(method, path, ts, body), sig).map_err(|_| AuthError::Signature)?;
    Ok(id)
}

/// Sessione ospitata, come la vedono gli amici.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Hosting {
    /// Gioco o app in streaming, se noto.
    pub app: Option<String>,
    pub slots_free: u8,
    pub guests: Vec<String>,
}

/// Risposta di `GET /v1/presence`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Presence {
    /// `invisible` non viene mai inviato: chi è invisibile risulta offline.
    pub status: Option<Status>,
    /// Disponibile come host, anche senza sessione attiva.
    pub can_join: bool,
    pub hosting: Option<Hosting>,
    /// `updatedAt` del profilo, per sapere se riscaricarlo.
    pub profile_updated_at: i64,
    /// Porta base di Sunshine dell'host.
    pub sunshine_port: u16,
}

pub const DEFAULT_PRESENCE_PORT: u16 = 47900;

/// `POST /v1/handshake`: l'ospite conclude un invito (§6 passo 5).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HandshakeRequest {
    pub token: String,
    pub profile: crate::profile::SignedProfile,
    /// ID stabile del nodo Tailscale dell'ospite.
    pub node_id: String,
    pub presence_port: u16,
    /// Invito Tailscale inverso, per rendere la condivisione reciproca (§6 passo 9).
    pub reverse_invite_url: Option<String>,
}

/// Risposta dell'host: amicizia salvata, PIN per il pairing di Moonlight.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HandshakeResponse {
    pub profile: crate::profile::SignedProfile,
    pub pin: String,
    pub sunshine_port: u16,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signed_request_roundtrip() {
        let me = Identity::generate();
        let h = sign_request(&me, "get", "/v1/presence", 1000, b"");
        let id = verify_request(
            Some(&h[0].1),
            Some(&h[1].1),
            Some(&h[2].1),
            "GET",
            "/v1/presence",
            b"",
            1030,
        );
        assert_eq!(id, Ok(me.id()));
    }

    #[test]
    fn rejects_skew_tampering_and_missing() {
        let me = Identity::generate();
        let h = sign_request(&me, "POST", "/v1/friend/remove", 1000, b"{}");
        let v = |path: &str, body: &[u8], now| {
            verify_request(Some(&h[0].1), Some(&h[1].1), Some(&h[2].1), "POST", path, body, now)
        };
        assert_eq!(v("/v1/friend/remove", b"{}", 1061), Err(AuthError::Skew));
        assert_eq!(v("/v1/friend/remove", b"{ }", 1000), Err(AuthError::Signature));
        assert_eq!(v("/v1/profile", b"{}", 1000), Err(AuthError::Signature));
        assert_eq!(verify_request(None, None, None, "GET", "/", b"", 0), Err(AuthError::Missing));
    }
}
