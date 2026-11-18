//! Profilo firmato (§4).

use serde::{Deserialize, Serialize};

use crate::canonical::to_canonical_json;
use crate::identity::{FalcoId, Identity, IdentityError};

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ProfileError {
    #[error("il nome deve avere tra 2 e 24 caratteri")]
    NameLength,
    #[error("il nome contiene caratteri non ammessi")]
    NameChars,
    #[error("la chiave non corrisponde al profilo")]
    WrongKey,
    #[error(transparent)]
    Identity(#[from] IdentityError),
    #[error("serializzazione: {0}")]
    Json(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    #[default]
    Online,
    Busy,
    Invisible,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Device {
    pub node_id: String,
    pub name: String,
    pub os: String,
    pub can_host: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Profile {
    pub id: FalcoId,
    pub tag: String,
    pub name: String,
    /// SHA-256 esadecimale dell'avatar WebP, se presente.
    pub avatar: Option<String>,
    pub accent: String,
    pub status: Status,
    pub devices: Vec<Device>,
    /// Secondi Unix dell'ultima modifica.
    pub updated_at: i64,
}

impl Profile {
    /// Nuovo profilo con tag e accento derivati dall'ID.
    pub fn new(id: FalcoId, name: &str, updated_at: i64) -> Result<Self, ProfileError> {
        Ok(Self {
            tag: id.tag(),
            accent: id.accent(),
            id,
            name: validate_name(name)?,
            avatar: None,
            status: Status::Online,
            devices: Vec::new(),
            updated_at,
        })
    }

    pub fn sign(self, identity: &Identity) -> Result<SignedProfile, ProfileError> {
        if identity.id() != self.id {
            return Err(ProfileError::WrongKey);
        }
        let bytes = to_canonical_json(&self).map_err(|e| ProfileError::Json(e.to_string()))?;
        let sig = identity.sign(&bytes);
        Ok(SignedProfile { profile: self, sig })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SignedProfile {
    #[serde(flatten)]
    pub profile: Profile,
    pub sig: String,
}

impl SignedProfile {
    pub fn verify(&self) -> Result<&Profile, ProfileError> {
        let bytes =
            to_canonical_json(&self.profile).map_err(|e| ProfileError::Json(e.to_string()))?;
        self.profile.id.verify(&bytes, &self.sig)?;
        Ok(&self.profile)
    }

    /// Un aggiornamento vale solo se firmato e più recente della cache.
    pub fn supersedes(&self, cached: &Profile) -> bool {
        self.profile.id == cached.id
            && self.profile.updated_at > cached.updated_at
            && self.verify().is_ok()
    }
}

/// 2–24 caratteri Unicode, spazi ai bordi rimossi, niente caratteri di controllo.
pub fn validate_name(raw: &str) -> Result<String, ProfileError> {
    let name = raw.trim();
    if name.chars().any(char::is_control) {
        return Err(ProfileError::NameChars);
    }
    if !(2..=24).contains(&name.chars().count()) {
        return Err(ProfileError::NameLength);
    }
    Ok(name.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn name_rules() {
        assert_eq!(validate_name("  Marco ").unwrap(), "Marco");
        assert_eq!(validate_name("M"), Err(ProfileError::NameLength));
        assert_eq!(validate_name("Ma\u{7}rco"), Err(ProfileError::NameChars));
        assert!(validate_name("ÀÉÎÕÜ🔥").is_ok());
        assert_eq!(validate_name(&"x".repeat(25)), Err(ProfileError::NameLength));
    }

    #[test]
    fn signed_profile_verifies_and_detects_tampering() {
        let me = Identity::generate();
        let signed = Profile::new(me.id(), "Marco", 10).unwrap().sign(&me).unwrap();
        assert!(signed.verify().is_ok());

        let json = serde_json::to_string(&signed).unwrap();
        let mut back: SignedProfile = serde_json::from_str(&json).unwrap();
        assert!(back.verify().is_ok());
        back.profile.name = "Mallory".into();
        assert!(back.verify().is_err());
    }

    #[test]
    fn only_newer_signed_updates_supersede() {
        let me = Identity::generate();
        let old = Profile::new(me.id(), "Marco", 10).unwrap();
        let newer = Profile { updated_at: 20, ..old.clone() }.sign(&me).unwrap();
        let same = old.clone().sign(&me).unwrap();
        assert!(newer.supersedes(&old));
        assert!(!same.supersedes(&old));
    }

    #[test]
    fn foreign_key_cannot_sign() {
        let me = Identity::generate();
        let other = Identity::generate();
        let p = Profile::new(me.id(), "Marco", 1).unwrap();
        assert_eq!(p.sign(&other), Err(ProfileError::WrongKey));
    }
}
