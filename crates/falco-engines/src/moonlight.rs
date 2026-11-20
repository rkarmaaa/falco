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
        // Lo stream è incorporato in Falco: senza, SDL ignora i controller quando crede che la
        // sua finestra non sia in primo piano e all'host non arriva nessun comando
        "--background-gamepad".into(),
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
    /// Statistiche globali stampate da Moonlight a fine sessione.
    Summary(VideoStats),
}

/// «Global video stats» di moonlight-qt: la misura della latenza di ogni sessione.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VideoStats {
    pub network_fps: f32,
    pub decode_fps: f32,
    pub render_fps: f32,
    /// Cattura e codifica sull'host (media).
    pub host_latency_ms: f32,
    pub host_latency_max_ms: f32,
    pub network_latency_ms: f32,
    pub network_variance_ms: f32,
    pub decode_ms: f32,
    pub queue_ms: f32,
    /// Rendering, compresa l'attesa del V-sync del monitor.
    pub render_ms: f32,
    pub dropped_network_pct: f32,
    pub dropped_jitter_pct: f32,
}

impl VideoStats {
    /// Stima della latenza dallo schermo dell'host al monitor dell'ospite, senza la cattura.
    pub fn total_ms(&self) -> f32 {
        self.host_latency_ms
            + self.network_latency_ms
            + self.decode_ms
            + self.queue_ms
            + self.render_ms
    }
}

fn first_number(s: &str) -> Option<f32> {
    let start = s.find(|c: char| c.is_ascii_digit())?;
    let tail = &s[start..];
    let end = tail.find(|c: char| !(c.is_ascii_digit() || c == '.')).unwrap_or(tail.len());
    tail[..end].parse().ok()
}

/// Raccoglie il blocco «Global video stats» riga per riga.
#[derive(Debug, Default)]
pub struct StatsCollector {
    active: bool,
    stats: VideoStats,
}

impl StatsCollector {
    /// Restituisce le statistiche quando il blocco è completo.
    pub fn feed(&mut self, line: &str) -> Option<VideoStats> {
        let l = line.trim();
        if l.contains("Global video stats") {
            self.active = true;
            self.stats = VideoStats::default();
            return None;
        }
        if !self.active {
            return None;
        }
        let s = &mut self.stats;
        let (key, value) = l.split_once(':').unwrap_or((l, ""));
        let n = first_number(value);
        match key {
            "Incoming frame rate from network" => s.network_fps = n.unwrap_or(0.0),
            "Decoding frame rate" => s.decode_fps = n.unwrap_or(0.0),
            "Rendering frame rate" => s.render_fps = n.unwrap_or(0.0),
            "Host processing latency min/max/average" => {
                let parts: Vec<f32> = value
                    .trim()
                    .trim_end_matches("ms")
                    .split('/')
                    .filter_map(|p| p.trim().parse().ok())
                    .collect();
                if let [_, max, avg] = parts[..] {
                    s.host_latency_max_ms = max;
                    s.host_latency_ms = avg;
                }
            }
            "Frames dropped by your network connection" => s.dropped_network_pct = n.unwrap_or(0.0),
            "Frames dropped due to network jitter" => s.dropped_jitter_pct = n.unwrap_or(0.0),
            "Average network latency" => {
                s.network_latency_ms = n.unwrap_or(0.0);
                s.network_variance_ms =
                    value.split("variance:").nth(1).and_then(first_number).unwrap_or(0.0);
            }
            "Average decoding time" => s.decode_ms = n.unwrap_or(0.0),
            "Average frame queue delay" => s.queue_ms = n.unwrap_or(0.0),
            k if k.starts_with("Average rendering time") => {
                s.render_ms = n.unwrap_or(0.0);
                // Ultima riga del blocco
                self.active = false;
                return Some(s.clone());
            }
            _ => {}
        }
        None
    }
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
    fn collects_global_video_stats() {
        let block = "00:00:14 - SDL Info (0): \nGlobal video stats\n------------------\nIncoming frame rate from network: 60.14 FPS\nDecoding frame rate: 60.14 FPS\nRendering frame rate: 59.94 FPS\nHost processing latency min/max/average: 2.6/24.8/2.8 ms\nFrames dropped by your network connection: 0.00%\nFrames dropped due to network jitter: 0.33%\nAverage network latency: 1 ms (variance: 0 ms)\nAverage decoding time: 0.21 ms\nAverage frame queue delay: 0.01 ms\nAverage rendering time (including monitor V-sync latency): 0.32 ms\n00:00:14 - SDL Info (0): Stopping input stream...";
        let mut c = StatsCollector::default();
        let s = block.lines().find_map(|l| c.feed(l)).unwrap();
        assert!(
            (s.host_latency_ms - 2.8).abs() < 1e-3 && (s.host_latency_max_ms - 24.8).abs() < 1e-3
        );
        assert!((s.network_latency_ms - 1.0).abs() < 1e-3);
        assert!((s.dropped_jitter_pct - 0.33).abs() < 1e-3);
        assert!((s.total_ms() - 4.34).abs() < 0.01);
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
