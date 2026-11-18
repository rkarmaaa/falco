//! Identità locale (§4): chiave Ed25519 nel portachiavi, profilo in SQLite, avatar WebP.

use std::io::Cursor;

use anyhow::{Context, bail};
use data_encoding::{BASE64, BASE64URL_NOPAD, HEXLOWER};
use falco_proto::profile::{Device, Profile, SignedProfile, validate_name};
use falco_proto::{Identity, Status};
use falco_store::Db;
use image::imageops::FilterType;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

const KEY_SECRET: &str = "identity-key";
const META_PROFILE: &str = "profile";
const AVATAR_SIZE: u32 = 256;
const AVATAR_MAX_BYTES: usize = 64 * 1024;

/// Parte modificabile del profilo, salvata in `meta`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Stored {
    name: String,
    avatar: Option<String>,
    updated_at: i64,
}

pub struct Me {
    pub identity: Identity,
    stored: Stored,
}

impl Me {
    /// Carica l'identità; `None` se l'utente non ha ancora fatto l'onboarding.
    pub fn load(db: &Db) -> anyhow::Result<Option<Self>> {
        let Some(identity) = load_key()? else { return Ok(None) };
        let Some(json) = db.meta(META_PROFILE)? else { return Ok(None) };
        Ok(Some(Self { identity, stored: serde_json::from_str(&json)? }))
    }

    /// Prima configurazione, nome validato. Una chiave già nel portachiavi si riusa:
    /// rigenerarla cambierebbe l'ID e gli amici non ci riconoscerebbero più.
    pub fn create(db: &Db, name: &str, now: i64) -> anyhow::Result<Self> {
        let name = validate_name(name)?;
        let identity = match load_key()? {
            Some(identity) => identity,
            None => {
                let identity = Identity::generate();
                crate::secrets::set(KEY_SECRET, &BASE64URL_NOPAD.encode(&identity.secret_bytes()))?;
                identity
            }
        };
        let me = Self { identity, stored: Stored { name, avatar: None, updated_at: now } };
        me.save(db)?;
        Ok(me)
    }

    fn save(&self, db: &Db) -> anyhow::Result<()> {
        db.set_meta(META_PROFILE, &serde_json::to_string(&self.stored)?)?;
        Ok(())
    }

    pub fn name(&self) -> &str {
        &self.stored.name
    }

    pub fn avatar_hash(&self) -> Option<&str> {
        self.stored.avatar.as_deref()
    }

    pub fn updated_at(&self) -> i64 {
        self.stored.updated_at
    }

    pub fn rename(&mut self, db: &Db, name: &str, now: i64) -> anyhow::Result<()> {
        self.stored.name = validate_name(name)?;
        self.stored.updated_at = now;
        self.save(db)
    }

    /// Immagine scelta dall'utente → avatar 256×256 WebP sotto 64 KB, senza metadati.
    pub fn set_avatar(&mut self, db: &Db, image_bytes: &[u8], now: i64) -> anyhow::Result<String> {
        let webp = encode_avatar(image_bytes)?;
        let hash = HEXLOWER.encode(&Sha256::digest(&webp));
        db.put_avatar(&hash, &webp)?;
        self.stored.avatar = Some(hash.clone());
        self.stored.updated_at = now;
        self.save(db)?;
        Ok(hash)
    }

    pub fn clear_avatar(&mut self, db: &Db, now: i64) -> anyhow::Result<()> {
        self.stored.avatar = None;
        self.stored.updated_at = now;
        self.save(db)
    }

    /// Profilo firmato per gli amici. Chi è invisibile non lo rivela nel profilo.
    pub fn signed(&self, status: Status, devices: Vec<Device>) -> anyhow::Result<SignedProfile> {
        let id = self.identity.id();
        let profile = Profile {
            tag: id.tag(),
            accent: id.accent(),
            id,
            name: self.stored.name.clone(),
            avatar: self.stored.avatar.clone(),
            status: if status == Status::Invisible { Status::Online } else { status },
            devices,
            updated_at: self.stored.updated_at,
        };
        Ok(profile.sign(&self.identity)?)
    }
}

fn load_key() -> anyhow::Result<Option<Identity>> {
    let Some(secret) = crate::secrets::get(KEY_SECRET)? else { return Ok(None) };
    let bytes: [u8; 32] = BASE64URL_NOPAD
        .decode(secret.as_bytes())
        .ok()
        .and_then(|b| b.try_into().ok())
        .context("chiave dell'identità danneggiata")?;
    Ok(Some(Identity::from_secret_bytes(bytes)))
}

/// Ritaglio centrale, ridimensionamento e WebP con qualità decrescente fino a stare nel limite.
pub fn encode_avatar(bytes: &[u8]) -> anyhow::Result<Vec<u8>> {
    let img = image::ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()?
        .decode()
        .context("immagine non leggibile")?;
    let side = img.width().min(img.height());
    if side == 0 {
        bail!("immagine vuota");
    }
    let x = (img.width() - side) / 2;
    let y = (img.height() - side) / 2;
    let square =
        img.crop_imm(x, y, side, side).resize_exact(AVATAR_SIZE, AVATAR_SIZE, FilterType::Lanczos3);
    let rgba = square.to_rgba8();
    for quality in [80.0, 70.0, 60.0, 50.0, 40.0, 30.0] {
        let out =
            webp::Encoder::from_rgba(&rgba, AVATAR_SIZE, AVATAR_SIZE).encode(quality).to_vec();
        if out.len() <= AVATAR_MAX_BYTES {
            return Ok(out);
        }
    }
    bail!("avatar troppo pesante anche alla qualità minima")
}

/// Data URL per la UI (gli avatar sono piccoli).
pub fn data_url(webp: &[u8]) -> String {
    format!("data:image/webp;base64,{}", BASE64.encode(webp))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn avatar_is_square_webp_under_limit() {
        // Immagine rumorosa 640×400: comprime male, verifica il limite
        let img = image::RgbImage::from_fn(640, 400, |x, y| {
            image::Rgb([((x * 7) ^ (y * 13)) as u8, (x * y) as u8, (x + y * 3) as u8])
        });
        let mut png = Vec::new();
        image::DynamicImage::ImageRgb8(img)
            .write_to(&mut Cursor::new(&mut png), image::ImageFormat::Png)
            .unwrap();
        let webp = encode_avatar(&png).unwrap();
        assert!(webp.len() <= AVATAR_MAX_BYTES);
        let back = image::load_from_memory(&webp).unwrap();
        assert_eq!((back.width(), back.height()), (256, 256));
    }

    #[test]
    fn rejects_garbage() {
        assert!(encode_avatar(b"non un'immagine").is_err());
    }
}
