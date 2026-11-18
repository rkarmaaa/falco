//! `config.toml` (§15): la UI è solo un editor di questo file.
//! Le modifiche passano da `toml_edit`, così commenti e chiavi sconosciute restano.

use std::collections::BTreeMap;
use std::io::Write;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use toml_edit::{DocumentMut, Item, Table};

pub const DEFAULT_TOML: &str = include_str!("default_config.toml");

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("config.toml, riga {line}: {message}")]
    Parse { line: usize, message: String },
    #[error("valore non valido per «{key}»: {value} (ammesso: {allowed}){}", line_hint(*line))]
    Invalid { key: String, value: String, allowed: String, line: Option<usize> },
    #[error("chiave sconosciuta: {0}")]
    UnknownKey(String),
    #[error("impossibile leggere o scrivere la configurazione: {0}")]
    Io(#[from] std::io::Error),
}

fn line_hint(line: Option<usize>) -> String {
    line.map(|l| format!(", riga {l}")).unwrap_or_default()
}

/// Quando ha effetto una modifica, mostrato in UI accanto al controllo.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ApplyAt {
    Now,
    NextSession,
    HostRestart,
}

/// `"auto"` oppure un valore esplicito.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Auto<T> {
    Auto(AutoLit),
    Value(T),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AutoLit {
    #[serde(rename = "auto")]
    Auto,
}

impl<T> Auto<T> {
    pub const AUTO: Self = Auto::Auto(AutoLit::Auto);
    pub fn value(&self) -> Option<&T> {
        match self {
            Auto::Value(v) => Some(v),
            Auto::Auto(_) => None,
        }
    }
}

macro_rules! str_enum {
    ($name:ident { $($variant:ident = $s:literal),+ $(,)? } default $def:ident) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
        pub enum $name {
            $( #[serde(rename = $s)] $variant, )+
        }
        impl $name {
            pub const ALL: &'static [&'static str] = &[$($s),+];
        }
        impl Default for $name { fn default() -> Self { Self::$def } }
    };
}

str_enum!(Language { It = "it", En = "en" } default It);
str_enum!(Updates { Auto = "auto", Notify = "notify", Off = "off" } default Auto);
str_enum!(Codec { Auto = "auto", H264 = "h264", Hevc = "hevc", Av1 = "av1" } default Auto);
str_enum!(BitrateMode { Auto = "auto", Fixed = "fixed" } default Auto);
str_enum!(DisplayMode { Fullscreen = "fullscreen", Borderless = "borderless", Windowed = "windowed" } default Borderless);
str_enum!(Audio { Stereo = "stereo", Surround51 = "5.1", Surround71 = "7.1" } default Stereo);
str_enum!(CaptureKeys { Never = "never", Fullscreen = "fullscreen", Always = "always" } default Fullscreen);
str_enum!(Overlay { Auto = "auto", Always = "always", Never = "never" } default Auto);
str_enum!(HostProfile { Latency = "latency", Balanced = "balanced", Slow = "slow", Quality = "quality", Custom = "custom" } default Latency);
str_enum!(AudioCapture { Onscreen = "onscreen", All = "all" } default Onscreen);
str_enum!(EncoderQuality { Speed = "speed", Balanced = "balanced", Quality = "quality" } default Speed);
str_enum!(Encoder { Auto = "auto", Amf = "amf", Nvenc = "nvenc", QuickSync = "quicksync", VideoToolbox = "videotoolbox", Software = "software" } default Auto);
str_enum!(Provider { Tailscale = "tailscale", Lan = "lan", Direct = "direct", Headscale = "headscale" } default Tailscale);
str_enum!(AddressFamily { Ipv4 = "ipv4", Ipv6 = "ipv6", Both = "both" } default Both);
str_enum!(FirewallScope { Tailnet = "tailnet", TailnetLan = "tailnet+lan" } default Tailnet);
str_enum!(LogLevel { Error = "error", Warn = "warn", Info = "info", Debug = "debug" } default Warn);

str_enum!(Status { Online = "online", Busy = "busy", Invisible = "invisible" } default Online);

pub const RESOLUTIONS: &[&str] = &["auto", "1280x720", "1920x1080", "2560x1440", "3840x2160"];
pub const FPS_VALUES: &[u16] = &[30, 60, 90, 120];
pub const ROLES: &[&str] = &["spettatore", "giocatore", "copilota", "fidato"];

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub schema: u32,
    pub general: General,
    pub profile: ProfileCfg,
    pub stream: Stream,
    pub host: Host,
    pub network: Network,
    pub performance: Performance,
    pub advanced: Advanced,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct General {
    pub language: Language,
    pub updates: Updates,
    pub start_with_system: bool,
    pub start_minimized: bool,
    pub close_to_tray: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ProfileCfg {
    pub status: Status,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Stream {
    pub resolution: String,
    pub fps: Auto<u16>,
    pub codec: Codec,
    pub bitrate_mode: BitrateMode,
    pub bitrate_kbps: u32,
    pub display_mode: DisplayMode,
    pub vsync: bool,
    pub frame_pacing: bool,
    pub audio: Audio,
    pub capture_system_keys: CaptureKeys,
    pub overlay: Overlay,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Host {
    pub available_on_start: bool,
    pub max_guests: u8,
    pub encoder: Encoder,
    pub default_role: String,
    pub knock_timeout_s: u32,
    pub apps_autodetect: bool,
    /// Profilo di condivisione: imposta bitrate massimo, FEC e qualità dell'encoder.
    pub profile: HostProfile,
    /// Tetto del bitrate per tutti gli ospiti (0 = nessun tetto). Solo con profilo «custom».
    pub max_bitrate_kbps: u32,
    pub encoder_quality: EncoderQuality,
    /// Audio condiviso: solo le app visibili sullo schermo condiviso (le chat vocali mai) o tutto il PC.
    pub audio_capture: AudioCapture,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Network {
    pub provider: Provider,
    pub presence_port: u16,
    pub sunshine_base_port: u16,
    pub address_family: AddressFamily,
    pub upnp: bool,
    pub fec_percent: u8,
    pub relay_bitrate_cap_kbps: u32,
    pub qos_dscp: bool,
    pub firewall_scope: FirewallScope,
    pub connect_timeout_s: u32,
    pub reconnect_attempts: u32,
    pub warn_exit_node: bool,
    pub headscale_url: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Performance {
    pub raise_priority: bool,
    pub warn_on_battery: bool,
    pub poll_ui_s: u32,
    pub poll_tray_s: u32,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Advanced {
    pub log_level: LogLevel,
    pub sunshine_overrides: BTreeMap<String, serde_json::Value>,
    pub moonlight_extra_args: Vec<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            schema: 1,
            general: General::default(),
            profile: ProfileCfg::default(),
            stream: Stream::default(),
            host: Host::default(),
            network: Network::default(),
            performance: Performance::default(),
            advanced: Advanced::default(),
        }
    }
}

impl Default for General {
    fn default() -> Self {
        Self {
            language: Language::It,
            updates: Updates::Auto,
            start_with_system: true,
            start_minimized: true,
            close_to_tray: true,
        }
    }
}

impl Default for Stream {
    fn default() -> Self {
        Self {
            resolution: "auto".into(),
            fps: Auto::AUTO,
            codec: Codec::Auto,
            bitrate_mode: BitrateMode::Auto,
            bitrate_kbps: 30_000,
            display_mode: DisplayMode::Fullscreen,
            vsync: false,
            frame_pacing: false,
            audio: Audio::Stereo,
            capture_system_keys: CaptureKeys::Fullscreen,
            overlay: Overlay::Auto,
        }
    }
}

impl Default for Host {
    fn default() -> Self {
        Self {
            available_on_start: false,
            max_guests: 3,
            encoder: Encoder::Auto,
            default_role: "spettatore".into(),
            knock_timeout_s: 30,
            apps_autodetect: true,
            profile: HostProfile::Latency,
            max_bitrate_kbps: 0,
            encoder_quality: EncoderQuality::Speed,
            audio_capture: AudioCapture::Onscreen,
        }
    }
}

impl Default for Network {
    fn default() -> Self {
        Self {
            provider: Provider::Tailscale,
            presence_port: 47900,
            sunshine_base_port: 47989,
            address_family: AddressFamily::Both,
            upnp: false,
            fec_percent: 20,
            relay_bitrate_cap_kbps: 8000,
            qos_dscp: true,
            firewall_scope: FirewallScope::Tailnet,
            connect_timeout_s: 10,
            reconnect_attempts: 3,
            warn_exit_node: true,
            headscale_url: String::new(),
        }
    }
}

impl Default for Performance {
    fn default() -> Self {
        Self { raise_priority: true, warn_on_battery: true, poll_ui_s: 15, poll_tray_s: 60 }
    }
}

struct Issue {
    key: &'static str,
    value: String,
    allowed: String,
}

fn check_range<T: PartialOrd + ToString>(
    out: &mut Vec<Issue>,
    key: &'static str,
    v: T,
    min: T,
    max: T,
) {
    if v < min || v > max {
        out.push(Issue {
            key,
            value: v.to_string(),
            allowed: format!("{}–{}", min.to_string(), max.to_string()),
        });
    }
}

impl Config {
    /// Controlli oltre al tipo: intervalli e valori ammessi.
    fn issues(&self) -> Vec<Issue> {
        let mut out = Vec::new();
        let s = &self.stream;
        if !RESOLUTIONS.contains(&s.resolution.as_str()) {
            out.push(Issue {
                key: "stream.resolution",
                value: s.resolution.clone(),
                allowed: RESOLUTIONS.join(", "),
            });
        }
        if let Some(fps) = s.fps.value()
            && !FPS_VALUES.contains(fps)
        {
            out.push(Issue {
                key: "stream.fps",
                value: fps.to_string(),
                allowed: "auto, 30, 60, 90, 120".into(),
            });
        }
        check_range(&mut out, "stream.bitrate_kbps", s.bitrate_kbps, 1_000, 150_000);
        let h = &self.host;
        check_range(&mut out, "host.max_guests", h.max_guests, 1, 8);
        check_range(&mut out, "host.knock_timeout_s", h.knock_timeout_s, 5, 300);
        if h.max_bitrate_kbps != 0 {
            check_range(&mut out, "host.max_bitrate_kbps", h.max_bitrate_kbps, 1_000, 150_000);
        }
        if !ROLES.contains(&h.default_role.as_str()) {
            out.push(Issue {
                key: "host.default_role",
                value: h.default_role.clone(),
                allowed: ROLES.join(", "),
            });
        }
        let n = &self.network;
        check_range(&mut out, "network.presence_port", n.presence_port, 1024, 65535);
        check_range(&mut out, "network.sunshine_base_port", n.sunshine_base_port, 1024, 65000);
        check_range(&mut out, "network.fec_percent", n.fec_percent, 0, 100);
        check_range(
            &mut out,
            "network.relay_bitrate_cap_kbps",
            n.relay_bitrate_cap_kbps,
            2_000,
            30_000,
        );
        check_range(&mut out, "network.connect_timeout_s", n.connect_timeout_s, 3, 60);
        check_range(&mut out, "network.reconnect_attempts", n.reconnect_attempts, 0, 10);
        if n.provider == Provider::Headscale && !n.headscale_url.starts_with("https://") {
            out.push(Issue {
                key: "network.headscale_url",
                value: format!("{:?}", n.headscale_url),
                allowed: "URL https:// del server Headscale".into(),
            });
        }
        let p = &self.performance;
        check_range(&mut out, "performance.poll_ui_s", p.poll_ui_s, 5, 300);
        check_range(&mut out, "performance.poll_tray_s", p.poll_tray_s, 15, 3600);
        out
    }
}

/// Momento di applicazione di una chiave.
pub fn apply_at(key: &str) -> ApplyAt {
    match key.split('.').next() {
        Some("stream") => ApplyAt::NextSession,
        Some("host") if key != "host.available_on_start" => ApplyAt::HostRestart,
        Some("advanced") if key.starts_with("advanced.sunshine") => ApplyAt::HostRestart,
        Some("network") => match key {
            "network.sunshine_base_port" | "network.upnp" | "network.fec_percent" => {
                ApplyAt::HostRestart
            }
            "network.relay_bitrate_cap_kbps" | "network.qos_dscp" => ApplyAt::NextSession,
            _ => ApplyAt::Now,
        },
        _ => ApplyAt::Now,
    }
}

/// Converte e valida un documento, con numero di riga negli errori.
pub fn parse(text: &str) -> Result<(DocumentMut, Config), ConfigError> {
    let doc: DocumentMut = text.parse().map_err(|e: toml_edit::TomlError| ConfigError::Parse {
        line: span_line(text, e.span()),
        message: e.message().to_owned(),
    })?;
    let config: Config = toml_edit::de::from_str(text).map_err(|e| ConfigError::Parse {
        line: span_line(text, e.span()),
        message: e.message().to_owned(),
    })?;
    if let Some(i) = config.issues().into_iter().next() {
        return Err(ConfigError::Invalid {
            line: find_line(text, i.key),
            key: i.key.into(),
            value: i.value,
            allowed: i.allowed,
        });
    }
    Ok((doc, config))
}

fn span_line(text: &str, span: Option<std::ops::Range<usize>>) -> usize {
    let at = span.map(|s| s.start).unwrap_or(0).min(text.len());
    text[..at].matches('\n').count() + 1
}

/// Riga di `section.key` nel testo, cercando la chiave dopo l'intestazione.
fn find_line(text: &str, dotted: &str) -> Option<usize> {
    let (section, key) = dotted.split_once('.')?;
    let header = format!("[{section}]");
    let mut in_section = false;
    for (i, line) in text.lines().enumerate() {
        let t = line.trim_start();
        if t.starts_with('[') {
            in_section = t.starts_with(&header);
        } else if in_section && t.strip_prefix(key).is_some_and(|r| r.trim_start().starts_with('='))
        {
            return Some(i + 1);
        }
    }
    None
}

/// File di configurazione caricato, con documento modificabile.
pub struct ConfigFile {
    path: PathBuf,
    doc: DocumentMut,
    config: Config,
}

impl ConfigFile {
    /// Carica il file o lo crea dal modello commentato.
    pub fn load_or_create(path: &Path) -> Result<Self, ConfigError> {
        let text = match std::fs::read_to_string(path) {
            Ok(t) => t,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                write_atomic(path, DEFAULT_TOML)?;
                DEFAULT_TOML.to_owned()
            }
            Err(e) => return Err(e.into()),
        };
        let (doc, config) = parse(&text)?;
        Ok(Self { path: path.to_owned(), doc, config })
    }

    pub fn in_memory(text: &str) -> Result<Self, ConfigError> {
        let (doc, config) = parse(text)?;
        Ok(Self { path: PathBuf::new(), doc, config })
    }

    pub fn config(&self) -> &Config {
        &self.config
    }

    pub fn text(&self) -> String {
        self.doc.to_string()
    }

    /// Imposta `sezione.chiave` (anche annidata) e salva. In caso di errore il file non cambia.
    pub fn set(&mut self, dotted: &str, value: &serde_json::Value) -> Result<ApplyAt, ConfigError> {
        let mut doc = self.doc.clone();
        let parts: Vec<&str> = dotted.split('.').collect();
        let Some((last, tables)) = parts.split_last() else {
            return Err(ConfigError::UnknownKey(dotted.into()));
        };
        if !Config::default().knows(&parts) {
            return Err(ConfigError::UnknownKey(dotted.into()));
        }
        let mut table: &mut dyn toml_edit::TableLike = doc.as_table_mut();
        for t in tables {
            let entry = table.entry(t).or_insert_with(|| Item::Table(Table::new()));
            table =
                entry.as_table_like_mut().ok_or_else(|| ConfigError::UnknownKey(dotted.into()))?;
        }
        let mut new_value = json_to_toml(value).ok_or_else(|| ConfigError::Invalid {
            key: dotted.into(),
            value: value.to_string(),
            allowed: "stringa, numero, booleano o elenco".into(),
            line: None,
        })?;
        // Mantiene il commento in linea della chiave
        if let Some(old) = table.get(last).and_then(Item::as_value) {
            *new_value.decor_mut() = old.decor().clone();
        }
        table.insert(last, Item::Value(new_value));
        self.commit(doc)?;
        Ok(apply_at(dotted))
    }

    /// Importa un file intero (Avanzate → Importa): validato prima di sostituire.
    pub fn replace_text(&mut self, text: &str) -> Result<(), ConfigError> {
        let doc: DocumentMut = text.parse().map_err(|e: toml_edit::TomlError| {
            ConfigError::Parse { line: span_line(text, e.span()), message: e.message().to_owned() }
        })?;
        self.commit(doc)
    }

    /// «Ripristina default» per un gruppo.
    pub fn reset_section(&mut self, section: &str) -> Result<(), ConfigError> {
        let defaults: DocumentMut =
            DEFAULT_TOML.parse().map_err(|_| ConfigError::UnknownKey(section.into()))?;
        let item = defaults
            .get(section)
            .cloned()
            .ok_or_else(|| ConfigError::UnknownKey(section.into()))?;
        let mut doc = self.doc.clone();
        doc.insert(section, item);
        self.commit(doc)
    }

    fn commit(&mut self, doc: DocumentMut) -> Result<(), ConfigError> {
        let text = doc.to_string();
        let (doc, config) = parse(&text)?;
        if !self.path.as_os_str().is_empty() {
            write_atomic(&self.path, &text)?;
        }
        self.doc = doc;
        self.config = config;
        Ok(())
    }
}

impl Config {
    /// La chiave esiste nello schema? `advanced.sunshine_overrides.*` è libera.
    fn knows(&self, parts: &[&str]) -> bool {
        if parts.len() >= 3 && parts[..2] == ["advanced", "sunshine_overrides"] {
            return true;
        }
        let Ok(json) = serde_json::to_value(self) else { return false };
        let mut cur = &json;
        for p in parts {
            match cur.get(p) {
                Some(next) => cur = next,
                None => return false,
            }
        }
        true
    }
}

fn json_to_toml(v: &serde_json::Value) -> Option<toml_edit::Value> {
    use serde_json::Value as J;
    Some(match v {
        J::Bool(b) => (*b).into(),
        J::Number(n) => match n.as_i64() {
            Some(i) => i.into(),
            None => n.as_f64()?.into(),
        },
        J::String(s) => s.as_str().into(),
        J::Array(items) => {
            let mut arr = toml_edit::Array::new();
            for it in items {
                arr.push(json_to_toml(it)?);
            }
            toml_edit::Value::Array(arr)
        }
        J::Object(map) => {
            let mut t = toml_edit::InlineTable::new();
            for (k, it) in map {
                t.insert(k, json_to_toml(it)?);
            }
            toml_edit::Value::InlineTable(t)
        }
        J::Null => return None,
    })
}

/// Scrittura atomica: file temporaneo nella stessa cartella, poi rename.
fn write_atomic(path: &Path, text: &str) -> Result<(), std::io::Error> {
    let dir = path.parent().unwrap_or(Path::new("."));
    std::fs::create_dir_all(dir)?;
    let mut tmp = tempfile::NamedTempFile::new_in(dir)?;
    tmp.write_all(text.as_bytes())?;
    tmp.as_file().sync_all()?;
    tmp.persist(path).map_err(|e| e.error)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn default_template_matches_default_struct() {
        let (_, cfg) = parse(DEFAULT_TOML).unwrap();
        assert_eq!(cfg, Config::default());
    }

    #[test]
    fn missing_sections_use_defaults() {
        let (_, cfg) = parse("schema = 1\n[stream]\nfps = 60\n").unwrap();
        assert_eq!(cfg.stream.fps, Auto::Value(60));
        assert_eq!(cfg.network.presence_port, 47900);
    }

    #[test]
    fn invalid_value_reports_key_and_line() {
        let text = DEFAULT_TOML.replace("fec_percent = 20", "fec_percent = 140");
        match parse(&text) {
            Err(ConfigError::Invalid { key, line, .. }) => {
                assert_eq!(key, "network.fec_percent");
                let l = line.unwrap();
                assert!(text.lines().nth(l - 1).unwrap().contains("fec_percent"));
            }
            other => panic!("atteso Invalid, ottenuto {other:?}"),
        }
    }

    #[test]
    fn wrong_enum_is_a_parse_error_with_line() {
        let text = DEFAULT_TOML.replace("codec = \"auto\"", "codec = \"vp9\"");
        assert!(matches!(parse(&text), Err(ConfigError::Parse { line, .. }) if line > 1));
    }

    #[test]
    fn set_preserves_comments_and_unknown_keys() {
        let text = format!("{DEFAULT_TOML}\n[futuro]\nopzione_nuova = 1\n");
        let mut f = ConfigFile::in_memory(&text).unwrap();
        let at = f.set("stream.fps", &json!(120)).unwrap();
        assert_eq!(at, ApplyAt::NextSession);
        assert_eq!(f.config().stream.fps, Auto::Value(120));
        let out = f.text();
        assert!(out.contains("opzione_nuova = 1"));
        assert!(
            out.lines()
                .any(|l| l.starts_with("fps = 120 ") && l.ends_with("# auto | 30 | 60 | 90 | 120")),
            "{out}"
        );
    }

    #[test]
    fn set_rejects_invalid_and_keeps_state() {
        let mut f = ConfigFile::in_memory(DEFAULT_TOML).unwrap();
        assert!(f.set("network.presence_port", &json!(80)).is_err());
        assert!(matches!(f.set("stream.nope", &json!(1)), Err(ConfigError::UnknownKey(_))));
        assert_eq!(f.config().network.presence_port, 47900);
        assert_eq!(f.text(), DEFAULT_TOML);
    }

    #[test]
    fn sunshine_overrides_are_free_form() {
        let mut f = ConfigFile::in_memory(DEFAULT_TOML).unwrap();
        f.set("advanced.sunshine_overrides.qp", &json!(28)).unwrap();
        assert!(f.config().advanced.sunshine_overrides.contains_key("qp"));
    }

    #[test]
    fn reset_section_and_file_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        let mut f = ConfigFile::load_or_create(&path).unwrap();
        f.set("general.language", &json!("en")).unwrap();
        assert!(std::fs::read_to_string(&path).unwrap().contains("language = \"en\""));
        f.reset_section("general").unwrap();
        assert_eq!(f.config().general.language, Language::It);
        let again = ConfigFile::load_or_create(&path).unwrap();
        assert_eq!(again.config(), f.config());
    }
}
