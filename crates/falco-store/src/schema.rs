//! Schema di `config.toml` per le impostazioni avanzate (§15): tipo, valori, intervalli,
//! momento di applicazione. La UI genera i controlli da qui.

use serde::Serialize;

use crate::config::{self, ApplyAt, apply_at};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase", tag = "type")]
pub enum Kind {
    Bool,
    Int {
        min: i64,
        max: i64,
    },
    Enum {
        options: Vec<String>,
    },
    /// `"auto"` oppure un numero tra le opzioni.
    AutoInt {
        options: Vec<i64>,
    },
    Text,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Field {
    pub key: &'static str,
    #[serde(flatten)]
    pub kind: Kind,
    pub apply_at: ApplyAt,
}

fn e(opts: &[&str]) -> Kind {
    Kind::Enum { options: opts.iter().map(|s| (*s).to_owned()).collect() }
}

fn i(min: i64, max: i64) -> Kind {
    Kind::Int { min, max }
}

/// Tutte le chiavi modificabili da Impostazioni → Avanzate, nell'ordine del §15.
pub fn fields() -> Vec<Field> {
    let list: Vec<(&'static str, Kind)> = vec![
        ("general.language", e(config::Language::ALL)),
        ("general.theme", e(config::Theme::ALL)),
        ("general.updates", e(config::Updates::ALL)),
        ("general.start_with_system", Kind::Bool),
        ("general.start_minimized", Kind::Bool),
        ("general.close_to_tray", Kind::Bool),
        ("stream.resolution", e(config::RESOLUTIONS)),
        (
            "stream.fps",
            Kind::AutoInt { options: config::FPS_VALUES.iter().map(|v| i64::from(*v)).collect() },
        ),
        ("stream.codec", e(config::Codec::ALL)),
        ("stream.bitrate_mode", e(config::BitrateMode::ALL)),
        ("stream.bitrate_kbps", i(1_000, 150_000)),
        ("stream.display_mode", e(config::DisplayMode::ALL)),
        ("stream.vsync", Kind::Bool),
        ("stream.frame_pacing", Kind::Bool),
        ("stream.audio", e(config::Audio::ALL)),
        ("stream.capture_system_keys", e(config::CaptureKeys::ALL)),
        ("stream.overlay", e(config::Overlay::ALL)),
        ("host.available_on_start", Kind::Bool),
        ("host.max_guests", i(1, 8)),
        ("host.encoder", e(config::Encoder::ALL)),
        ("host.default_role", e(config::ROLES)),
        ("host.knock_timeout_s", i(5, 300)),
        ("host.apps_autodetect", Kind::Bool),
        ("network.provider", e(config::Provider::ALL)),
        ("network.presence_port", i(1024, 65535)),
        ("network.sunshine_base_port", i(1024, 65000)),
        ("network.address_family", e(config::AddressFamily::ALL)),
        ("network.upnp", Kind::Bool),
        ("network.fec_percent", i(0, 100)),
        ("network.relay_bitrate_cap_kbps", i(2_000, 30_000)),
        ("network.qos_dscp", Kind::Bool),
        ("network.firewall_scope", e(config::FirewallScope::ALL)),
        ("network.connect_timeout_s", i(3, 60)),
        ("network.reconnect_attempts", i(0, 10)),
        ("network.warn_exit_node", Kind::Bool),
        ("network.headscale_url", Kind::Text),
        ("performance.raise_priority", Kind::Bool),
        ("performance.warn_on_battery", Kind::Bool),
        ("performance.poll_ui_s", i(5, 300)),
        ("performance.poll_tray_s", i(15, 3600)),
        ("advanced.log_level", e(config::LogLevel::ALL)),
    ];
    list.into_iter().map(|(key, kind)| Field { key, kind, apply_at: apply_at(key) }).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_field_exists_in_default_config() {
        let json = serde_json::to_value(crate::Config::default()).unwrap();
        for f in fields() {
            let (section, key) = f.key.split_once('.').unwrap();
            assert!(json[section].get(key).is_some(), "chiave assente: {}", f.key);
        }
    }

    #[test]
    fn every_config_key_is_in_schema() {
        let json = serde_json::to_value(crate::Config::default()).unwrap();
        let keys: Vec<&str> = fields().iter().map(|f| f.key).collect();
        for (section, v) in json.as_object().unwrap() {
            let Some(obj) = v.as_object() else { continue };
            for k in obj.keys() {
                let full = format!("{section}.{k}");
                let editable_elsewhere = matches!(
                    full.as_str(),
                    "profile.status"
                        | "advanced.sunshine_overrides"
                        | "advanced.moonlight_extra_args"
                );
                assert!(
                    editable_elsewhere || keys.contains(&full.as_str()),
                    "manca nello schema: {full}"
                );
            }
        }
    }
}
