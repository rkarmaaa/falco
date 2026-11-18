//! Segreti nel portachiavi di sistema (§11): Credential Manager su Windows, Keychain su macOS.

use anyhow::Context;

fn entry(name: &str) -> anyhow::Result<keyring::Entry> {
    let service = match falco_proto::endpoint::instance() {
        Some(i) => format!("app.falco.desktop.{i}"),
        None => "app.falco.desktop".into(),
    };
    keyring::Entry::new(&service, name).context("portachiavi non disponibile")
}

/// Legge un segreto, se esiste.
pub fn get(name: &str) -> anyhow::Result<Option<String>> {
    match entry(name)?.get_password() {
        Ok(p) => Ok(Some(p)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(e).context("lettura dal portachiavi"),
    }
}

pub fn set(name: &str, value: &str) -> anyhow::Result<()> {
    entry(name)?.set_password(value).context("segreto non salvato nel portachiavi")
}

/// Legge un segreto o ne crea uno casuale al primo uso.
pub fn get_or_create(name: &str) -> anyhow::Result<String> {
    let entry = entry(name)?;
    match entry.get_password() {
        Ok(p) => Ok(p),
        Err(keyring::Error::NoEntry) => {
            let (secret, _) = falco_proto::invite::new_token();
            entry.set_password(&secret).context("segreto non salvato nel portachiavi")?;
            Ok(secret)
        }
        Err(e) => Err(e).context("lettura dal portachiavi"),
    }
}
