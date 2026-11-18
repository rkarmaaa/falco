//! Integrazione con il sistema (§14): avvio con il sistema, schema `falco://`, Tailscale.

use anyhow::Context;

fn exe_path() -> anyhow::Result<String> {
    Ok(std::env::current_exe()?.to_string_lossy().into_owned())
}

/// Allinea l'avvio con il sistema a `general.start_with_system`.
pub fn sync_autostart(enabled: bool) -> anyhow::Result<()> {
    let al = auto_launch::AutoLaunchBuilder::new()
        .set_app_name("Falco")
        .set_app_path(&exe_path()?)
        .set_args(&["--autostart"])
        .build()?;
    if enabled && !al.is_enabled().unwrap_or(false) {
        al.enable().context("avvio con il sistema non attivato")?;
    } else if !enabled && al.is_enabled().unwrap_or(false) {
        al.disable()?;
    }
    Ok(())
}

/// Registra `falco://` per l'utente corrente (Windows). Su macOS lo fa `Info.plist`.
#[cfg(windows)]
pub fn register_url_scheme() -> anyhow::Result<()> {
    use winreg::RegKey;
    use winreg::enums::HKEY_CURRENT_USER;
    let exe = exe_path()?;
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let (root, _) = hkcu.create_subkey(r"Software\Classes\falco")?;
    root.set_value("", &"URL:Falco")?;
    root.set_value("URL Protocol", &"")?;
    let (icon, _) = root.create_subkey("DefaultIcon")?;
    icon.set_value("", &format!("\"{exe}\",0"))?;
    let (cmd, _) = root.create_subkey(r"shell\open\command")?;
    cmd.set_value("", &format!("\"{exe}\" --open-link \"%1\""))?;
    Ok(())
}

#[cfg(not(windows))]
pub fn register_url_scheme() -> anyhow::Result<()> {
    Ok(())
}

/// URL ufficiali del client Tailscale (ADR 0012, opzione A).
#[cfg(windows)]
const TAILSCALE_URL: &str = "https://pkgs.tailscale.com/stable/tailscale-setup-latest-amd64.msi";
#[cfg(not(windows))]
const TAILSCALE_URL: &str = "https://pkgs.tailscale.com/stable/Tailscale-latest-macos.pkg";

/// Installa Tailscale senza procedure manuali: download del pacchetto ufficiale firmato
/// e installazione silenziosa con una sola conferma di sistema (UAC o password).
pub async fn install_tailscale(download_dir: &std::path::Path) -> anyhow::Result<()> {
    let ext = if cfg!(windows) { "msi" } else { "pkg" };
    let file = download_dir.join(format!("tailscale-setup.{ext}"));
    let f2 = file.clone();
    tokio::task::spawn_blocking(move || -> anyhow::Result<()> {
        let mut resp =
            ureq::get(TAILSCALE_URL).call().context("download di Tailscale non riuscito")?;
        let mut out = std::fs::File::create(&f2)?;
        std::io::copy(&mut resp.body_mut().as_reader(), &mut out)?;
        Ok(())
    })
    .await??;
    verify_signature(&file).await?;
    let path = file.to_string_lossy().into_owned();
    let status = if cfg!(windows) {
        // Una richiesta UAC; l'MSI ufficiale avvia il servizio da solo
        let script = format!(
            "Start-Process msiexec -Verb RunAs -Wait -ArgumentList '/i \"{path}\" /quiet /norestart'"
        );
        tokio::process::Command::new("powershell")
            .args(["-NoProfile", "-Command", &script])
            .status()
            .await?
    } else {
        let script = format!(
            "do shell script \"installer -pkg '{path}' -target /\" with administrator privileges"
        );
        tokio::process::Command::new("osascript").args(["-e", &script]).status().await?
    };
    anyhow::ensure!(status.success(), "installazione di Tailscale annullata o non riuscita");
    Ok(())
}

/// Il pacchetto deve essere firmato da Tailscale (Authenticode su Windows, notarizzazione su macOS).
async fn verify_signature(file: &std::path::Path) -> anyhow::Result<()> {
    let path = file.to_string_lossy().into_owned();
    let ok = if cfg!(windows) {
        let script = format!(
            "$s = Get-AuthenticodeSignature '{path}'; $s.Status -eq 'Valid' -and $s.SignerCertificate.Subject -like '*Tailscale*'"
        );
        let out = tokio::process::Command::new("powershell")
            .args(["-NoProfile", "-Command", &script])
            .output()
            .await?;
        String::from_utf8_lossy(&out.stdout).trim().eq_ignore_ascii_case("true")
    } else {
        let out = tokio::process::Command::new("pkgutil")
            .args(["--check-signature", &path])
            .output()
            .await?;
        String::from_utf8_lossy(&out.stdout).contains("Tailscale")
    };
    anyhow::ensure!(ok, "il pacchetto di Tailscale non ha una firma valida");
    Ok(())
}
