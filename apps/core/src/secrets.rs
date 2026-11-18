//! Segreti nel portachiavi di sistema (§11): Credential Manager su Windows, Keychain su macOS.

use anyhow::Context;

const SERVICE: &str = "app.falo.desktop";

/// Legge un segreto o ne crea uno casuale al primo uso.
pub fn get_or_create(name: &str) -> anyhow::Result<String> {
    let entry = keyring::Entry::new(SERVICE, name).context("portachiavi non disponibile")?;
    match entry.get_password() {
        Ok(p) => Ok(p),
        Err(keyring::Error::NoEntry) => {
            let (secret, _) = falo_proto::invite::new_token();
            entry.set_password(&secret).context("segreto non salvato nel portachiavi")?;
            Ok(secret)
        }
        Err(e) => Err(e).context("lettura dal portachiavi"),
    }
}
