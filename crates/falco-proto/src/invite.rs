//! Link di invito firmati (§6): JSON canonico → firma → zstd → base64url.

use data_encoding::{BASE64URL_NOPAD, HEXLOWER};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::canonical::to_canonical_json;
use crate::identity::{FalcoId, Identity};
use crate::permissions::Role;

pub const SCHEME_PREFIX: &str = "falco://invite/";
const MAX_DECODED: usize = 16 * 1024;

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum InviteError {
    #[error("non è un invito di Falco")]
    Malformed,
    #[error("versione dell'invito non supportata")]
    Version,
    #[error("firma dell'invito non valida")]
    Signature,
    #[error("invito scaduto")]
    Expired,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InviteHost {
    pub id: FalcoId,
    pub name: String,
    pub tag: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InviteTailscale {
    pub invite_url: String,
    pub node: String,
    pub ip: String,
    /// Porta di presenza, solo se diversa da 47900.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub port: Option<u16>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InvitePayload {
    pub v: u8,
    pub host: InviteHost,
    pub ts: InviteTailscale,
    pub token: String,
    pub role: Role,
    pub exp: i64,
}

#[derive(Serialize, Deserialize)]
struct SignedInvite {
    #[serde(flatten)]
    payload: InvitePayload,
    sig: String,
}

/// Token casuale di 32 byte e il suo SHA-256: solo l'hash va salvato.
pub fn new_token() -> (String, String) {
    let raw: [u8; 32] = rand::random();
    let token = BASE64URL_NOPAD.encode(&raw);
    let hash = token_hash(&token);
    (token, hash)
}

pub fn token_hash(token: &str) -> String {
    HEXLOWER.encode(&Sha256::digest(token.as_bytes()))
}

/// Firma e codifica il payload come link `falco://invite/…`.
pub fn encode(payload: InvitePayload, host: &Identity) -> Result<String, InviteError> {
    let bytes = to_canonical_json(&payload).map_err(|_| InviteError::Malformed)?;
    let signed = SignedInvite { sig: host.sign(&bytes), payload };
    let json = serde_json::to_vec(&signed).map_err(|_| InviteError::Malformed)?;
    let packed = zstd::bulk::compress(&json, 19).map_err(|_| InviteError::Malformed)?;
    Ok(format!("{SCHEME_PREFIX}{}", BASE64URL_NOPAD.encode(&packed)))
}

/// Verifica firma e scadenza. Accetta il link completo o solo la parte codificata.
pub fn decode(link: &str, now: i64) -> Result<InvitePayload, InviteError> {
    let body = link.trim().strip_prefix(SCHEME_PREFIX).unwrap_or(link.trim());
    let packed = BASE64URL_NOPAD
        .decode(body.trim_end_matches('/').as_bytes())
        .map_err(|_| InviteError::Malformed)?;
    let json = zstd::bulk::decompress(&packed, MAX_DECODED).map_err(|_| InviteError::Malformed)?;
    let signed: SignedInvite = serde_json::from_slice(&json).map_err(|_| InviteError::Malformed)?;
    if signed.payload.v != 1 {
        return Err(InviteError::Version);
    }
    let bytes = to_canonical_json(&signed.payload).map_err(|_| InviteError::Malformed)?;
    signed.payload.host.id.verify(&bytes, &signed.sig).map_err(|_| InviteError::Signature)?;
    if now >= signed.payload.exp {
        return Err(InviteError::Expired);
    }
    Ok(signed.payload)
}

/// Cerca un link di invito in un testo qualsiasi (appunti, messaggio incollato).
pub fn find_in_text(text: &str) -> Option<&str> {
    let start = text.find(SCHEME_PREFIX)?;
    let rest = &text[start..];
    let end = rest
        .find(|c: char| c.is_whitespace() || matches!(c, '"' | '\'' | '<' | '>' | ')'))
        .unwrap_or(rest.len());
    Some(&rest[..end])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(host: &Identity, exp: i64) -> InvitePayload {
        InvitePayload {
            v: 1,
            host: InviteHost { id: host.id(), name: "Marco".into(), tag: host.id().tag() },
            ts: InviteTailscale {
                invite_url: "https://login.tailscale.com/admin/invite/abc".into(),
                node: "marco-pc".into(),
                ip: "100.101.102.103".into(),
                port: None,
            },
            token: new_token().0,
            role: Role::Player,
            exp,
        }
    }

    #[test]
    fn roundtrip() {
        let host = Identity::generate();
        let payload = sample(&host, 1_000);
        let link = encode(payload.clone(), &host).unwrap();
        assert!(link.starts_with(SCHEME_PREFIX));
        assert_eq!(decode(&link, 10).unwrap(), payload);
    }

    #[test]
    fn expired_and_tampered() {
        let host = Identity::generate();
        let link = encode(sample(&host, 1_000), &host).unwrap();
        assert_eq!(decode(&link, 1_000), Err(InviteError::Expired));
        assert_eq!(decode("falco://invite/AAAA", 0), Err(InviteError::Malformed));

        // Payload firmato da un'altra chiave: host.id non corrisponde
        let mallory = Identity::generate();
        let mut forged = sample(&host, 1_000);
        forged.role = Role::Trusted;
        let link = encode(forged, &mallory).unwrap();
        assert_eq!(decode(&link, 0), Err(InviteError::Signature));
    }

    #[test]
    fn finds_link_in_chat_text() {
        let text = "ehi entra qui: falco://invite/abcDEF_-12 dai!";
        assert_eq!(find_in_text(text), Some("falco://invite/abcDEF_-12"));
        assert_eq!(find_in_text("niente qui"), None);
    }

    #[test]
    fn token_hash_is_stable() {
        let (t, h) = new_token();
        assert_eq!(token_hash(&t), h);
        assert_eq!(h.len(), 64);
    }
}
