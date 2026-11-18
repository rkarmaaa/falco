//! API di Falco Hub (ADR 0017): codici amico, richieste, link di stream, chiavi di rete.
//! Le richieste sono firmate come FPP ([`crate::fpp::sign_request`]).

use serde::{Deserialize, Serialize};

use crate::identity::FalcoId;

pub const ADD_PREFIX: &str = "falco://add/";
pub const JOIN_PREFIX: &str = "falco://join/";
/// Validità degli inviti allegati alle richieste d'amicizia, in ore.
pub const REQUEST_VALIDITY_H: u32 = 168;
const CODE_LEN: usize = 8;

/// Codice amico: il tag dell'ID senza `#` e `-` (`K7QM2XPA`).
pub fn code_of(id: &FalcoId) -> String {
    id.tag().chars().filter(char::is_ascii_alphanumeric).collect()
}

/// `K7QM2XPA` → `K7QM-2XPA`, come lo mostra la UI.
pub fn display_code(code: &str) -> String {
    if code.len() == CODE_LEN { format!("{}-{}", &code[..4], &code[4..]) } else { code.to_owned() }
}

/// Accetta `#k7qm-2xpa`, `K7QM 2XPA`, `k7qm2xpa`: restituisce la forma canonica.
pub fn normalize_code(s: &str) -> Option<String> {
    let code: String =
        s.chars().filter(char::is_ascii_alphanumeric).map(|c| c.to_ascii_uppercase()).collect();
    let base32 = |c: char| c.is_ascii_uppercase() || ('2'..='7').contains(&c);
    (code.len() == CODE_LEN && code.chars().all(base32)).then_some(code)
}

/// Codice corto casuale dei link di stream (base32, 8 caratteri = 40 bit).
pub fn new_link_code() -> String {
    let bytes: [u8; 5] = rand::random();
    data_encoding::BASE32_NOPAD.encode(&bytes)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HubLink {
    /// Aggiungi un amico per codice.
    Add(String),
    /// Entra nello stream di un host.
    Join(String),
}

/// Riconosce un link del Hub in un testo: `falco://add/…`, `https://…/a/…`, `falco://join/…`,
/// `https://…/j/…`, oppure un codice amico nudo.
pub fn parse_link(text: &str) -> Option<HubLink> {
    let t = text.trim();
    let tail = |prefix: &str| {
        t.find(prefix).map(|i| {
            t[i + prefix.len()..]
                .split(|c: char| !c.is_ascii_alphanumeric() && c != '-')
                .next()
                .unwrap_or_default()
                .to_owned()
        })
    };
    if let Some(c) = tail(ADD_PREFIX).or_else(|| tail("/a/")) {
        return normalize_code(&c).map(HubLink::Add);
    }
    if let Some(c) = tail(JOIN_PREFIX).or_else(|| tail("/j/")) {
        return normalize_code(&c).map(HubLink::Join);
    }
    normalize_code(t).map(HubLink::Add)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Register {
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Registered {
    pub code: String,
    pub friend_link: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserRef {
    pub id: String,
    pub name: String,
    pub code: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FriendRequest {
    pub id: String,
    pub user: UserRef,
    pub created_at: i64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Requests {
    pub incoming: Vec<FriendRequest>,
    pub outgoing: Vec<FriendRequest>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SendRequest {
    pub code: String,
    /// Invito firmato a uso singolo di chi chiede.
    pub invite: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Accepted {
    pub invite: String,
    pub user: UserRef,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateLink {
    pub invite: String,
    /// Scadenza dell'invito: il link non dura di più.
    pub expires_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Link {
    pub code: String,
    pub url: String,
    pub expires_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NetKey {
    /// Server di controllo; vuoto = quello di Tailscale.
    #[serde(default)]
    pub login_server: String,
    pub auth_key: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identity::Identity;

    #[test]
    fn code_matches_tag() {
        let id = Identity::generate().id();
        let code = code_of(&id);
        assert_eq!(format!("#{}", display_code(&code)), id.tag());
        assert_eq!(normalize_code(&id.tag().to_lowercase()), Some(code));
    }

    #[test]
    fn rejects_bad_codes() {
        assert_eq!(normalize_code("K7QM-2XP"), None);
        assert_eq!(normalize_code("K7QM-2XP1"), None, "1 non è base32");
    }

    #[test]
    fn parses_links() {
        assert_eq!(parse_link("falco://add/K7QM2XPA"), Some(HubLink::Add("K7QM2XPA".into())));
        assert_eq!(
            parse_link("guarda: https://hub.example/j/ABCD2345 ciao"),
            Some(HubLink::Join("ABCD2345".into()))
        );
        assert_eq!(parse_link("#k7qm-2xpa"), Some(HubLink::Add("K7QM2XPA".into())));
        assert_eq!(parse_link("falco://invite/xyz"), None);
    }
}
