//! Ospite: pairing e streaming con Moonlight (§8.2). La UI si chiude durante il gioco.

use std::net::IpAddr;
use std::time::Duration;

use anyhow::{Context, bail};
use falco_engines::moonlight::{
    self, AutoInputs, EngineEvent, NetPath, StreamRequest, VideoCodec, auto_params,
};
use falco_engines::process::{EngineProcess, Priority, command};
use falco_proto::ipc::events;
use falco_store::config::{
    Audio, Auto, CaptureKeys, Codec, Config, DisplayMode, Overlay, RESOLUTIONS,
};
use serde_json::json;

use crate::core::{Shared, now};

/// Indirizzo per Moonlight: la porta solo se diversa da quella standard.
pub fn moonlight_address(host: &str, port: u16) -> String {
    if port == 47989 || host.contains(':') && !host.contains(']') && host.matches(':').count() == 1
    {
        host.to_owned()
    } else {
        format!("{host}:{port}")
    }
}

/// L'host è questo stesso PC? (prova locale e loopback)
pub fn is_local(host: &str, self_ips: &[IpAddr]) -> bool {
    let h = host.split(':').next().unwrap_or(host);
    h == "localhost"
        || h.parse::<IpAddr>().is_ok_and(|ip| ip.is_loopback() || self_ips.contains(&ip))
}

fn primary_screen() -> ((u32, u32), u16) {
    display_info::DisplayInfo::all()
        .ok()
        .and_then(|all| all.into_iter().find(|d| d.is_primary))
        // display-info restituisce già pixel fisici: nessuna moltiplicazione per la scala
        .map(|d| ((d.width.min(3840), d.height.min(2160)), d.frequency.round() as u16))
        .unwrap_or(((1920, 1080), 60))
}

fn parse_res(s: &str) -> Option<(u32, u32)> {
    let (w, h) = s.split_once('x')?;
    Some((w.parse().ok()?, h.parse().ok()?))
}

/// Parametri automatici (§8.2) corretti dalle preferenze in `[stream]`.
pub fn plan(cfg: &Config, host: &str, path: NetPath, app: &str) -> StreamRequest {
    let (screen, hz) = primary_screen();
    let s = &cfg.stream;
    let fixed_res =
        RESOLUTIONS.contains(&s.resolution.as_str()).then(|| parse_res(&s.resolution)).flatten();
    let auto = auto_params(&AutoInputs {
        host_screen: fixed_res.unwrap_or(screen),
        guest_screen: screen,
        guest_refresh_hz: hz,
        max_resolution: fixed_res,
        max_fps: match s.fps {
            Auto::Value(f) => Some(f),
            Auto::Auto(_) => None,
        },
        max_bitrate_kbps: None,
        path,
        host_av1_hw: false,
        guest_av1_hw: false,
        guest_hevc_hw: true,
    });
    let codec = match s.codec {
        Codec::Auto => auto.codec,
        Codec::H264 => VideoCodec::H264,
        Codec::Hevc => VideoCodec::Hevc,
        Codec::Av1 => VideoCodec::Av1,
    };
    let bitrate = match s.bitrate_mode {
        falco_store::config::BitrateMode::Fixed => s.bitrate_kbps,
        falco_store::config::BitrateMode::Auto => auto.bitrate_kbps,
    };
    StreamRequest {
        host: host.to_owned(),
        app: app.to_owned(),
        width: auto.width,
        height: auto.height,
        fps: match s.fps {
            Auto::Value(f) => f,
            Auto::Auto(_) => auto.fps,
        },
        bitrate_kbps: bitrate,
        codec,
        display_mode: match s.display_mode {
            DisplayMode::Fullscreen => "fullscreen",
            DisplayMode::Borderless => "borderless",
            DisplayMode::Windowed => "windowed",
        }
        .into(),
        vsync: s.vsync,
        frame_pacing: s.frame_pacing,
        audio: match s.audio {
            Audio::Stereo => "stereo",
            Audio::Surround51 => "5.1-surround",
            Audio::Surround71 => "7.1-surround",
        }
        .into(),
        capture_system_keys: match s.capture_system_keys {
            CaptureKeys::Never => "never",
            CaptureKeys::Fullscreen => "fullscreen",
            CaptureKeys::Always => "always",
        }
        .into(),
        performance_overlay: s.overlay == Overlay::Always,
        extra_args: cfg.advanced.moonlight_extra_args.clone(),
    }
}

/// PIN casuale di 4 cifre.
pub fn new_pin() -> String {
    let n = u16::from_be_bytes(rand_bytes()) % 10_000;
    format!("{n:04}")
}

fn rand_bytes() -> [u8; 2] {
    let (t, _) = falco_proto::invite::new_token();
    let b = t.as_bytes();
    [b[0], b[1]]
}

/// Pairing lato ospite. Se l'host è questo PC, il core conferma anche il PIN.
pub async fn pair(core: Shared, address: String, pin: String) -> anyhow::Result<()> {
    let bin = falco_engines::locate::moonlight().context("motore client non trovato")?;
    let args = moonlight::pair_args(&address, &pin);
    let mut child = command(&bin, &args, Priority::Normal).spawn().context("avvio del pairing")?;

    let local_api = {
        let c = core.lock().await;
        let ips = c.last_self_ips.clone();
        if is_local(&address, &ips) { c.host.api() } else { None }
    };
    if let Some(api) = local_api {
        // Conferma automatica: riprova ogni 500 ms per 30 s (§6 passo 8)
        let pin2 = pin.clone();
        tokio::task::spawn_blocking(move || {
            for _ in 0..60 {
                if let Ok(list) = api.pending_pairings()
                    && let Some(p) = list.first()
                    && api.submit_pin(&p.id, &pin2, "Questo PC").is_ok()
                {
                    tracing::info!("pairing locale confermato");
                    return;
                }
                std::thread::sleep(Duration::from_millis(500));
            }
        });
    }

    // moonlight-qt resta aperto dopo il pairing: l'esito si verifica con `list`
    let mut ok = false;
    for attempt in 0..60 {
        tokio::time::sleep(Duration::from_secs(2)).await;
        let apps = list_apps(&bin, &address).await;
        tracing::info!("pairing {address}: tentativo {attempt}, app {apps:?}");
        if !apps.is_empty() {
            ok = true;
            break;
        }
        if let Ok(Some(st)) = child.try_wait() {
            tracing::info!("pairing {address}: moonlight uscito ({st})");
            // Uscito: ultimo controllo dopo che Moonlight ha salvato il certificato
            tokio::time::sleep(Duration::from_secs(1)).await;
            ok = !list_apps(&bin, &address).await.is_empty();
            break;
        }
    }
    let _ = child.kill().await;
    core.lock().await.emit(events::PAIR_RESULT, json!({ "host": address, "ok": ok }));
    if !ok {
        bail!("pairing non riuscito");
    }
    Ok(())
}

/// App disponibili sull'host; vuoto se non associato o non raggiungibile.
/// moonlight-qt può restare aperto dopo aver stampato l'elenco: si legge riga per riga
/// e si chiude il processo appena l'elenco è finito.
pub async fn list_apps(bin: &std::path::Path, address: &str) -> Vec<String> {
    use tokio::io::{AsyncBufReadExt, BufReader};
    let mut cmd = command(bin, &moonlight::list_args(address), Priority::Normal);
    cmd.stdout(std::process::Stdio::piped()).stderr(std::process::Stdio::piped());
    let Ok(mut child) = cmd.spawn() else { return Vec::new() };
    let Some(out) = child.stdout.take() else { return Vec::new() };
    // Moonlight scrive i log su stderr: va svuotato o il processo si blocca
    if let Some(err) = child.stderr.take() {
        tokio::spawn(async move {
            let mut lines = BufReader::new(err).lines();
            while let Ok(Some(_)) = lines.next_line().await {}
        });
    }
    let mut lines = BufReader::new(out).lines();
    let mut apps = Vec::new();
    let deadline = tokio::time::Instant::now() + Duration::from_secs(10);
    loop {
        // Dopo la prima app basta una breve pausa senza righe nuove
        let wait = if apps.is_empty() {
            deadline
        } else {
            tokio::time::Instant::now() + Duration::from_millis(400)
        };
        match tokio::time::timeout_at(wait.min(deadline), lines.next_line()).await {
            Ok(Ok(Some(line))) => apps.extend(moonlight::parse_list(&line)),
            _ => break,
        }
    }
    let _ = child.kill().await;
    apps
}

/// Avvia lo streaming; ritorna subito, la sessione prosegue in background.
pub async fn join(core: Shared, address: String, app: String) -> anyhow::Result<()> {
    let bin = falco_engines::locate::moonlight().context("motore client non trovato")?;
    let (cfg, ts) = {
        let c = core.lock().await;
        if c.streaming {
            bail!("c'è già una sessione attiva");
        }
        (c.config.config().clone(), c.net.clone())
    };
    let net = crate::core::Core::net_status(ts).await;
    let req = {
        let mut c = core.lock().await;
        let path = c.path_to(&address, &net);
        c.streaming = true;
        c.close_ui();
        plan(&cfg, &address, path, &app)
    };
    let args = moonlight::stream_args(&req);
    tracing::info!("stream: {}", args.join(" "));
    let (mut child, mut events) =
        EngineProcess::spawn(&bin, &args, Priority::AboveNormal)?.into_parts();
    let started = now();

    tokio::spawn(async move {
        let mut reason = "closed".to_owned();
        loop {
            tokio::select! {
                ev = events.recv() => match ev {
                    Some(EngineEvent::Connecting) => core.lock().await.emit(events::SESSION_STATS, json!({ "phase": "connecting" })),
                    Some(EngineEvent::Connected) => core.lock().await.emit(events::SESSION_STATS, json!({ "phase": "connected" })),
                    Some(EngineEvent::Stats { rtt_ms, fps, bitrate_kbps, path }) => core.lock().await.emit(
                        events::SESSION_STATS,
                        json!({ "phase": "stats", "rttMs": rtt_ms, "fps": fps, "bitrateKbps": bitrate_kbps, "path": path }),
                    ),
                    // Moonlight resta aperto con un messaggio: lo chiudiamo noi
                    Some(EngineEvent::Ended { reason: r }) => {
                        reason = r;
                        let _ = child.kill().await;
                        break;
                    }
                    None => {}
                },
                _ = child.wait() => break,
            }
        }
        let mut c = core.lock().await;
        c.streaming = false;
        // Avvisa l'host che abbiamo lasciato la sessione
        if let (Some(fid), Some(me)) = (c.joined.take(), c.me.as_ref().map(|m| m.identity.clone()))
        {
            let addr = c.live.get(&fid).and_then(|l| l.address.clone());
            if let Some(addr) = addr {
                tokio::task::spawn_blocking(move || {
                    crate::social::fpp_call(
                        &me,
                        "POST",
                        &addr,
                        "/v1/session/leave",
                        Some(&json!({})),
                        None,
                        Duration::from_secs(3),
                    )
                });
            }
        }
        let _ = c.db.log_session(&falco_store::db::SessionRow {
            friend_id: address.clone(),
            role: "guest".into(),
            started_at: started,
            ended_at: now(),
            avg_rtt_ms: None,
            path: "direct".into(),
        });
        c.emit(
            events::SESSION_ENDED,
            json!({ "host": address, "reason": reason, "durationS": now() - started }),
        );
        c.open_ui();
    });
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn addresses_and_local_detection() {
        assert_eq!(moonlight_address("100.64.0.2", 47989), "100.64.0.2");
        assert_eq!(moonlight_address("100.64.0.2", 49989), "100.64.0.2:49989");
        assert_eq!(moonlight_address("localhost:49989", 49989), "localhost:49989");
        let me: IpAddr = "100.70.2.107".parse().unwrap();
        assert!(is_local("127.0.0.1:49989", &[]));
        assert!(is_local("100.70.2.107", &[me]));
        assert!(!is_local("100.64.0.9", &[me]));
    }

    #[test]
    fn pin_has_four_digits() {
        for _ in 0..20 {
            let p = new_pin();
            assert_eq!(p.len(), 4);
            assert!(p.chars().all(|c| c.is_ascii_digit()));
        }
    }
}
