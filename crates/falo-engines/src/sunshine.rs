//! Host: generazione di `sunshine.conf` e `apps.json` (§8.1).
//! I nomi delle chiavi vanno verificati sulla versione del fork (spike M0, ADR 0006).

use std::collections::BTreeMap;
use std::fmt::Write;

use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GpuVendor {
    Amd,
    Nvidia,
    Intel,
    Apple,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Platform {
    Windows,
    MacOs,
}

impl Platform {
    pub fn current() -> Self {
        if cfg!(target_os = "macos") { Self::MacOs } else { Self::Windows }
    }
}

/// Impostazioni dell'host già validate dal core (da `config.toml`).
#[derive(Debug, Clone)]
pub struct HostSettings {
    /// `auto` | `amf` | `nvenc` | `quicksync` | `videotoolbox` | `software`
    pub encoder: String,
    pub max_guests: u8,
    pub upnp: bool,
    pub fec_percent: u8,
    pub base_port: u16,
    pub overrides: BTreeMap<String, String>,
}

/// Nome dell'encoder per Sunshine. `auto` sceglie in base alla GPU.
pub fn encoder_name(pref: &str, gpu: GpuVendor) -> Option<&'static str> {
    Some(match (pref, gpu) {
        ("amf", _) | ("auto", GpuVendor::Amd) => "amdvce",
        ("nvenc", _) | ("auto", GpuVendor::Nvidia) => "nvenc",
        ("quicksync", _) | ("auto", GpuVendor::Intel) => "quicksync",
        ("videotoolbox", _) | ("auto", GpuVendor::Apple) => "videotoolbox",
        ("software", _) => "software",
        _ => return None, // lascia decidere a Sunshine
    })
}

/// Genera `sunshine.conf`. Gli override dell'utente vincono sui valori generati.
pub fn generate_conf(s: &HostSettings, gpu: GpuVendor, platform: Platform) -> String {
    let mut kv: BTreeMap<String, String> = BTreeMap::new();
    let mut put = |k: &str, v: String| {
        kv.insert(k.to_owned(), v);
    };
    if let Some(enc) = encoder_name(&s.encoder, gpu) {
        put("encoder", enc.into());
    }
    put("port", s.base_port.to_string());
    put("channels", s.max_guests.to_string());
    put("upnp", if s.upnp { "enabled" } else { "disabled" }.into());
    put("fec_percentage", s.fec_percent.to_string());
    put("origin_web_ui_allowed", "pc".into());
    put("hevc_mode", "0".into()); // 0 = decide la capacità del client
    put("av1_mode", "0".into());
    put("min_log_level", "warning".into());
    if gpu == GpuVendor::Amd || s.encoder == "amf" {
        put("amd_usage", "ultralowlatency".into());
        put("amd_rc", "cbr".into());
        put("amd_quality", "speed".into());
        put("amd_preanalysis", "disabled".into());
    }
    match platform {
        Platform::MacOs => put("vt_realtime", "enabled".into()),
        Platform::Windows => put("gamepad", "x360".into()),
    }
    for (k, v) in &s.overrides {
        kv.insert(k.clone(), v.clone());
    }
    let mut out =
        String::from("# Generato da Falò: non modificare, usa advanced.sunshine_overrides\n");
    for (k, v) in kv {
        let _ = writeln!(out, "{k} = {v}");
    }
    out
}

#[derive(Debug, Clone, Serialize)]
pub struct AppEntry {
    pub name: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub cmd: String,
    #[serde(rename = "image-path", skip_serializing_if = "String::is_empty")]
    pub image_path: String,
}

impl AppEntry {
    pub fn desktop() -> Self {
        Self { name: "Desktop".into(), cmd: String::new(), image_path: String::new() }
    }
}

/// `apps.json` nel formato di Sunshine: «Desktop» sempre per primo.
pub fn generate_apps(apps: &[AppEntry]) -> serde_json::Value {
    let mut list = vec![AppEntry::desktop()];
    list.extend(apps.iter().filter(|a| a.name != "Desktop").cloned());
    serde_json::json!({ "env": {}, "apps": list })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn settings() -> HostSettings {
        HostSettings {
            encoder: "auto".into(),
            max_guests: 3,
            upnp: false,
            fec_percent: 20,
            base_port: 47989,
            overrides: BTreeMap::new(),
        }
    }

    #[test]
    fn amd_windows_defaults() {
        let conf = generate_conf(&settings(), GpuVendor::Amd, Platform::Windows);
        for line in [
            "encoder = amdvce",
            "channels = 3",
            "upnp = disabled",
            "fec_percentage = 20",
            "amd_usage = ultralowlatency",
            "amd_rc = cbr",
            "gamepad = x360",
            "origin_web_ui_allowed = pc",
            "min_log_level = warning",
        ] {
            assert!(conf.lines().any(|l| l == line), "manca «{line}»\n{conf}");
        }
        assert!(!conf.contains("vt_realtime"));
    }

    #[test]
    fn mac_has_no_gamepad_and_realtime_vt() {
        let conf = generate_conf(&settings(), GpuVendor::Apple, Platform::MacOs);
        assert!(conf.contains("encoder = videotoolbox"));
        assert!(conf.contains("vt_realtime = enabled"));
        assert!(!conf.contains("gamepad"));
        assert!(!conf.contains("amd_"));
    }

    #[test]
    fn overrides_win() {
        let mut s = settings();
        s.overrides.insert("fec_percentage".into(), "30".into());
        let conf = generate_conf(&s, GpuVendor::Nvidia, Platform::Windows);
        assert!(conf.contains("fec_percentage = 30"));
        assert!(!conf.contains("fec_percentage = 20"));
    }

    #[test]
    fn apps_start_with_desktop() {
        let apps = generate_apps(&[AppEntry {
            name: "Steam Big Picture".into(),
            cmd: "steam://open/bigpicture".into(),
            image_path: String::new(),
        }]);
        assert_eq!(apps["apps"][0]["name"], "Desktop");
        assert_eq!(apps["apps"][1]["cmd"], "steam://open/bigpicture");
    }
}
