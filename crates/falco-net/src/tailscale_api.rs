//! Tailscale API (§6): token OAuth client credentials e inviti di condivisione del dispositivo.
//! Chiamate bloccanti: il core le esegue fuori dal runtime.

use std::time::Duration;

use serde::Deserialize;

const API: &str = "https://api.tailscale.com/api/v2";

#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    #[error("Tailscale API: {0}")]
    Http(#[from] ureq::Error),
    #[error("risposta inattesa dalla Tailscale API")]
    Unexpected,
}

#[derive(Deserialize)]
struct Token {
    access_token: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Invite {
    id: String,
    invite_url: String,
}

/// Invito di condivisione creato per un dispositivo.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceInvite {
    pub id: String,
    pub url: String,
}

pub struct TailscaleApi {
    agent: ureq::Agent,
    token: String,
}

impl TailscaleApi {
    /// Scambia ID e secret dell'OAuth client per un token d'accesso.
    pub fn login(client_id: &str, client_secret: &str) -> Result<Self, ApiError> {
        let agent = ureq::Agent::config_builder()
            .timeout_global(Some(Duration::from_secs(15)))
            .build()
            .new_agent();
        let token: Token = agent
            .post(format!("{API}/oauth/token"))
            .send_form([("client_id", client_id), ("client_secret", client_secret)])?
            .body_mut()
            .read_json()?;
        Ok(Self { agent, token: token.access_token })
    }

    /// `POST /device/{id}/device-invites`: un link di condivisione, senza exit node.
    pub fn create_device_invite(
        &self,
        device_id: &str,
        multi_use: bool,
    ) -> Result<DeviceInvite, ApiError> {
        let invites: Vec<Invite> = self
            .agent
            .post(format!("{API}/device/{device_id}/device-invites"))
            .header("Authorization", &format!("Bearer {}", self.token))
            .send_json(serde_json::json!([{ "multiUse": multi_use, "allowExitNode": false }]))?
            .body_mut()
            .read_json()?;
        let i = invites.into_iter().next().ok_or(ApiError::Unexpected)?;
        Ok(DeviceInvite { id: i.id, url: i.invite_url })
    }

    /// Revoca un invito non ancora usato.
    pub fn delete_device_invite(&self, invite_id: &str) -> Result<(), ApiError> {
        self.agent
            .delete(format!("{API}/device-invites/{invite_id}"))
            .header("Authorization", &format!("Bearer {}", self.token))
            .call()?;
        Ok(())
    }
}
