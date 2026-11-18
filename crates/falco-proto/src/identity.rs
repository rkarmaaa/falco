//! Identità Ed25519 (§4): la chiave pubblica è l'ID, nessun account Falco.

use std::fmt;
use std::str::FromStr;

use data_encoding::{BASE32_NOPAD, BASE64URL_NOPAD};
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum IdentityError {
    #[error("ID non valido: deve essere una chiave Ed25519 in base32")]
    InvalidId,
    #[error("firma non valida")]
    BadSignature,
}

/// Chiave pubblica Ed25519, mostrata in base32 senza padding.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct FalcoId([u8; 32]);

impl FalcoId {
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    /// Impronta breve per la UI, formato `#K7QM-2XPA`.
    pub fn tag(&self) -> String {
        let digest = BASE32_NOPAD.encode(&Sha256::digest(self.0));
        format!("#{}-{}", &digest[0..4], &digest[4..8])
    }

    /// Colore d'accento stabile, usato per il monogramma senza avatar.
    pub fn accent(&self) -> String {
        let h = Sha256::digest(self.0);
        let hue = f32::from(u16::from_be_bytes([h[0], h[1]]) % 360);
        hsl_to_hex(hue, 0.62, 0.56)
    }

    /// Verifica una firma Ed25519 codificata base64url.
    pub fn verify(&self, message: &[u8], sig_b64: &str) -> Result<(), IdentityError> {
        let key = VerifyingKey::from_bytes(&self.0).map_err(|_| IdentityError::InvalidId)?;
        let bytes =
            BASE64URL_NOPAD.decode(sig_b64.as_bytes()).map_err(|_| IdentityError::BadSignature)?;
        let sig = Signature::from_slice(&bytes).map_err(|_| IdentityError::BadSignature)?;
        key.verify(message, &sig).map_err(|_| IdentityError::BadSignature)
    }
}

impl fmt::Display for FalcoId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&BASE32_NOPAD.encode(&self.0))
    }
}

impl fmt::Debug for FalcoId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "FalcoId({})", self.tag())
    }
}

impl FromStr for FalcoId {
    type Err = IdentityError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let bytes = BASE32_NOPAD.decode(s.as_bytes()).map_err(|_| IdentityError::InvalidId)?;
        let arr: [u8; 32] = bytes.try_into().map_err(|_| IdentityError::InvalidId)?;
        VerifyingKey::from_bytes(&arr).map_err(|_| IdentityError::InvalidId)?;
        Ok(Self(arr))
    }
}

impl TryFrom<String> for FalcoId {
    type Error = IdentityError;
    fn try_from(s: String) -> Result<Self, Self::Error> {
        s.parse()
    }
}

impl From<FalcoId> for String {
    fn from(id: FalcoId) -> Self {
        id.to_string()
    }
}

/// Coppia di chiavi locale. La parte privata va solo nel portachiavi (M2).
#[derive(Clone)]
pub struct Identity {
    key: SigningKey,
}

impl Identity {
    pub fn generate() -> Self {
        Self::from_secret_bytes(rand::random())
    }

    pub fn from_secret_bytes(secret: [u8; 32]) -> Self {
        Self { key: SigningKey::from_bytes(&secret) }
    }

    pub fn secret_bytes(&self) -> [u8; 32] {
        self.key.to_bytes()
    }

    pub fn id(&self) -> FalcoId {
        FalcoId(self.key.verifying_key().to_bytes())
    }

    /// Firma e restituisce la firma in base64url.
    pub fn sign(&self, message: &[u8]) -> String {
        BASE64URL_NOPAD.encode(&self.key.sign(message).to_bytes())
    }
}

fn hsl_to_hex(h: f32, s: f32, l: f32) -> String {
    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
    let m = l - c / 2.0;
    let (r, g, b) = match h as u32 {
        0..=59 => (c, x, 0.0),
        60..=119 => (x, c, 0.0),
        120..=179 => (0.0, c, x),
        180..=239 => (0.0, x, c),
        240..=299 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    let to = |v: f32| ((v + m) * 255.0).round().clamp(0.0, 255.0) as u8;
    format!("#{:02X}{:02X}{:02X}", to(r), to(g), to(b))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn id_roundtrip_and_tag_format() {
        let me = Identity::generate();
        let id = me.id();
        let parsed: FalcoId = id.to_string().parse().unwrap();
        assert_eq!(id, parsed);
        let tag = id.tag();
        assert_eq!(tag.len(), 10);
        assert!(tag.starts_with('#') && tag.chars().nth(5) == Some('-'));
        assert!(id.accent().starts_with('#') && id.accent().len() == 7);
    }

    #[test]
    fn sign_and_verify() {
        let me = Identity::generate();
        let sig = me.sign(b"ciao");
        assert!(me.id().verify(b"ciao", &sig).is_ok());
        assert_eq!(me.id().verify(b"ciaO", &sig), Err(IdentityError::BadSignature));
    }

    #[test]
    fn rejects_garbage_ids() {
        assert!("non-un-id".parse::<FalcoId>().is_err());
        assert!("AAAA".parse::<FalcoId>().is_err());
    }
}
