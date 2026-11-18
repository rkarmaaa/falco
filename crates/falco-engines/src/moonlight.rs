//! Client: argomenti della CLI di Moonlight, parametri automatici e stato (§8.2).
//! Flag verificati su moonlight-qt 6.2.0 (ADR 0007).

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum VideoCodec {
    H264,
    Hevc,
    Av1,
}

impl VideoCodec {
    fn flag(self) -> &'static str {
        match self {
            Self::H264 => "H.264",
            Self::Hevc => "HEVC",
            Self::Av1 => "AV1",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StreamRequest {
    pub host: String,
    pub app: String,
    pub width: u32,
    pub height: u32,
    pub fps: u16,
    pub bitrate_kbps: u32,
    pub codec: VideoCodec,
    /// `fullscreen` | `borderless` | `windowed`
    pub display_mode: String,
    pub vsync: bool,
    pub frame_pacing: bool,
    /// `stereo` | `5.1-surround` | `7.1-surround`
    pub audio: String,
    /// `never` | `fullscreen` | `always`
    pub capture_system_keys: String,
    pub performance_overlay: bool,
    pub extra_args: Vec<String>,
}

pub fn stream_args(r: &StreamRequest) -> Vec<String> {
    let mut a: Vec<String> = vec![
        "stream".into(),
        r.host.clone(),
        r.app.clone(),
        "--resolution".into(),
        format!("{}x{}", r.width, r.height),
        "--fps".into(),
        r.fps.to_string(),
        "--bitrate".into(),
        r.bitrate_kbps.to_string(),
        "--video-codec".into(),
        r.codec.flag().into(),
        "--display-mode".into(),
        r.display_mode.clone(),
        if r.vsync { "--vsync" } else { "--no-vsync" }.into(),
        if r.frame_pacing { "--frame-pacing" } else { "--no-frame-pacing" }.into(),
        "--audio-config".into(),
        r.audio.clone(),
        "--capture-system-keys".into(),
        r.capture_system_keys.clone(),
        if r.performance_overlay { "--performance-overlay" } else { "--no-performance-overlay" }
            .into(),
        "--game-optimization".into(),
        "--keep-awake".into(),
        "--multi-controller".into(),
        "--quit-after".into(),
    ];
    a.extend(r.extra_args.iter().cloned());
    a
}

pub fn pair_args(host: &str, pin: &str) -> Vec<String> {
    vec!["pair".into(), host.into(), "--pin".into(), pin.into()]
}

pub fn list_args(host: &str) -> Vec<String> {
    vec!["list".into(), host.into()]
}

/// App dell'host dall'output di `moonlight list`, escluse le righe di log.
pub fn parse_list(output: &str) -> Vec<String> {
    output
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .filter(|l| !(l.len() > 11 && l.as_bytes()[2] == b':' && l.contains(" - ")))
        .map(str::to_owned)
        .collect()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetPath {
    Lan,
    Internet { host_upload_kbps: u32 },
    Relay { cap_kbps: u32 },
}

/// Ingressi per i «Parametri automatici».
#[derive(Debug, Clone)]
pub struct AutoInputs {
    pub host_screen: (u32, u32),
    pub guest_screen: (u32, u32),
    pub guest_refresh_hz: u16,
    pub max_resolution: Option<(u32, u32)>,
    pub max_fps: Option<u16>,
    pub max_bitrate_kbps: Option<u32>,
    pub path: NetPath,
    pub host_av1_hw: bool,
    pub guest_av1_hw: bool,
    pub guest_hevc_hw: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AutoParams {
    pub width: u32,
    pub height: u32,
    pub fps: u16,
    pub codec: VideoCodec,
    pub bitrate_kbps: u32,
}

fn smaller(a: (u32, u32), b: (u32, u32)) -> (u32, u32) {
    if u64::from(a.0) * u64::from(a.1) <= u64::from(b.0) * u64::from(b.1) { a } else { b }
}

pub fn auto_params(i: &AutoInputs) -> AutoParams {
    let mut res = smaller(i.host_screen, i.guest_screen);
    if let Some(m) = i.max_resolution {
        res = smaller(res, m);
    }
    let mut fps = i.guest_refresh_hz.min(120);
    if let Some(m) = i.max_fps {
        fps = fps.min(m);
    }
    let codec = if i.host_av1_hw && i.guest_av1_hw {
        VideoCodec::Av1
    } else if i.guest_hevc_hw {
        VideoCodec::Hevc
    } else {
        VideoCodec::H264
    };
    let mut bitrate = match i.path {
        NetPath::Lan => 50_000,
        NetPath::Internet { host_upload_kbps } => host_upload_kbps * 6 / 10,
        NetPath::Relay { cap_kbps } => cap_kbps,
    };
    if let Some(m) = i.max_bitrate_kbps {
        bitrate = bitrate.min(m);
    }
    AutoParams { width: res.0, height: res.1, fps, codec, bitrate_kbps: bitrate.max(1_000) }
}

/// Stato emesso dal fork su stdout, una riga JSON per evento.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "event", rename_all = "lowercase")]
pub enum EngineEvent {
    Connecting,
    Connected,
    Stats {
        rtt_ms: f32,
        fps: f32,
        bitrate_kbps: u32,
        /// `direct` | `relay`
        path: String,
    },
    Ended {
        reason: String,
    },
}

/// Righe JSON del fork, oppure i log di moonlight-qt ufficiale come ripiego.
pub fn parse_event(line: &str) -> Option<EngineEvent> {
    let line = line.trim();
    if line.starts_with('{') {
        return serde_json::from_str(line).ok();
    }
    if line.contains("Starting RTSP handshake") {
        Some(EngineEvent::Connecting)
    } else if line.contains("Starting input stream") {
        Some(EngineEvent::Connected)
    } else if line.contains("Connection terminated") || line.contains("Starting connection failed")
    {
        Some(EngineEvent::Ended { reason: line.rsplit(": ").next().unwrap_or(line).to_owned() })
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn inputs() -> AutoInputs {
        AutoInputs {
            host_screen: (2560, 1440),
            guest_screen: (1920, 1080),
            guest_refresh_hz: 144,
            max_resolution: None,
            max_fps: None,
            max_bitrate_kbps: None,
            path: NetPath::Lan,
            host_av1_hw: true,
            guest_av1_hw: false,
            guest_hevc_hw: true,
        }
    }

    #[test]
    fn spec_example_args() {
        let a = stream_args(&StreamRequest {
            host: "100.101.102.103".into(),
            app: "Desktop".into(),
            width: 1920,
            height: 1080,
            fps: 60,
            bitrate_kbps: 30_000,
            codec: VideoCodec::Hevc,
            display_mode: "borderless".into(),
            vsync: false,
            frame_pacing: true,
            audio: "stereo".into(),
            capture_system_keys: "fullscreen".into(),
            performance_overlay: false,
            extra_args: vec![],
        });
        let joined = a.join(" ");
        assert!(
            joined.starts_with("stream 100.101.102.103 Desktop --resolution 1920x1080 --fps 60")
        );
        assert!(joined.contains("--video-codec HEVC") && joined.ends_with("--quit-after"));
    }

    #[test]
    fn auto_lan_m1_mac() {
        let p = auto_params(&inputs());
        assert_eq!((p.width, p.height, p.fps), (1920, 1080, 120));
        assert_eq!(p.codec, VideoCodec::Hevc, "M1 non decodifica AV1 in hardware");
        assert_eq!(p.bitrate_kbps, 50_000);
    }

    #[test]
    fn auto_respects_permissions_and_relay() {
        let mut i = inputs();
        i.max_resolution = Some((1280, 720));
        i.max_fps = Some(60);
        i.path = NetPath::Relay { cap_kbps: 8_000 };
        i.guest_av1_hw = true;
        let p = auto_params(&i);
        assert_eq!((p.width, p.height, p.fps, p.bitrate_kbps), (1280, 720, 60, 8_000));
        assert_eq!(p.codec, VideoCodec::Av1);
        i.path = NetPath::Internet { host_upload_kbps: 40_000 };
        assert_eq!(auto_params(&i).bitrate_kbps, 24_000);
    }

    #[test]
    fn list_output_skips_logs() {
        let out = "00:00:00 - Qt Info: Discovered mDNS host
Desktop
Steam Big Picture
";
        assert_eq!(parse_list(out), ["Desktop", "Steam Big Picture"]);
    }

    #[test]
    fn parses_engine_events() {
        assert_eq!(parse_event(r#"{"event":"connected"}"#), Some(EngineEvent::Connected));
        let s = parse_event(
            r#"{"event":"stats","rtt_ms":12.5,"fps":60,"bitrate_kbps":28000,"path":"direct"}"#,
        );
        assert!(matches!(s, Some(EngineEvent::Stats { bitrate_kbps: 28000, .. })));
        assert_eq!(parse_event("Qt: qualcosa"), None);
        assert_eq!(
            parse_event("00:00:03 - SDL Info (0): Starting RTSP handshake..."),
            Some(EngineEvent::Connecting)
        );
        assert_eq!(
            parse_event("00:00:03 - SDL Info (0): Starting input stream..."),
            Some(EngineEvent::Connected)
        );
    }
}
