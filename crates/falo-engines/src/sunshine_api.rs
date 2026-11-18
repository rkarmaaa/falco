//! Client dell'API REST locale di Sunshine (verificata su Sunshine 2026.914, ADR 0006).
//! Solo `localhost`, certificato autofirmato, autenticazione Basic.

use std::time::Duration;

use data_encoding::BASE64;
use serde::{Deserialize, Serialize};
use ureq::Agent;
use ureq::tls::TlsConfig;

#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    #[error("Sunshine non risponde: {0}")]
    Http(#[from] ureq::Error),
    #[error("Sunshine ha rifiutato la richiesta")]
    Rejected,
}

/// Richiesta di associazione in attesa del PIN.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PendingPairing {
    pub id: String,
    pub name: String,
    pub address: String,
}

/// Client associato (certificato di Moonlight).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PairedClient {
    pub uuid: String,
    pub name: String,
    pub enabled: bool,
}

#[derive(Deserialize)]
struct Pairings {
    #[serde(default)]
    pairings: Vec<PendingPairing>,
}

#[derive(Deserialize)]
struct Clients {
    #[serde(default)]
    named_certs: Vec<PairedClient>,
}

#[derive(Deserialize)]
struct Status {
    status: bool,
}

pub struct SunshineApi {
    agent: Agent,
    base: String,
    auth: String,
}

impl SunshineApi {
    /// `web_port` è la porta base + 1 (47990 di default).
    pub fn new(web_port: u16, user: &str, password: &str) -> Self {
        let agent = Agent::config_builder()
            .tls_config(TlsConfig::builder().disable_verification(true).build())
            .timeout_global(Some(Duration::from_secs(5)))
            .build()
            .new_agent();
        Self {
            agent,
            base: format!("https://localhost:{web_port}/api"),
            auth: format!("Basic {}", BASE64.encode(format!("{user}:{password}").as_bytes())),
        }
    }

    fn get<T: for<'de> Deserialize<'de>>(&self, path: &str) -> Result<T, ApiError> {
        Ok(self
            .agent
            .get(format!("{}{path}", self.base))
            .header("Authorization", &self.auth)
            .call()?
            .body_mut()
            .read_json()?)
    }

    fn post(&self, path: &str, body: serde_json::Value) -> Result<(), ApiError> {
        let s: Status = self
            .agent
            .post(format!("{}{path}", self.base))
            .header("Authorization", &self.auth)
            .send_json(body)?
            .body_mut()
            .read_json()?;
        if s.status { Ok(()) } else { Err(ApiError::Rejected) }
    }

    /// Sunshine è pronto quando l'API risponde.
    pub fn ready(&self) -> bool {
        self.clients().is_ok()
    }

    pub fn pending_pairings(&self) -> Result<Vec<PendingPairing>, ApiError> {
        Ok(self.get::<Pairings>("/pin")?.pairings)
    }

    /// Conferma una richiesta con il PIN mostrato dall'ospite.
    pub fn submit_pin(&self, pairing_id: &str, pin: &str, name: &str) -> Result<(), ApiError> {
        self.post("/pin", serde_json::json!({ "pairing_id": pairing_id, "pin": pin, "name": name }))
    }

    pub fn clients(&self) -> Result<Vec<PairedClient>, ApiError> {
        Ok(self.get::<Clients>("/clients/list")?.named_certs)
    }

    pub fn unpair(&self, uuid: &str) -> Result<(), ApiError> {
        self.post("/clients/unpair", serde_json::json!({ "uuid": uuid }))
    }

    /// Abilita o blocca un client senza revocarne il pairing.
    pub fn set_enabled(&self, uuid: &str, enabled: bool) -> Result<(), ApiError> {
        self.post("/clients/update", serde_json::json!({ "uuid": uuid, "enabled": enabled }))
    }

    /// Chiude l'app in streaming (termina la sessione).
    pub fn close_app(&self) -> Result<(), ApiError> {
        self.post("/apps/close", serde_json::json!({}))
    }
}
