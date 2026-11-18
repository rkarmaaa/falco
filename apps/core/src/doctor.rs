//! Dottore di rete completo (§9): un semaforo e un'azione per ogni controllo.

use std::net::{IpAddr, SocketAddr};
use std::time::{Duration, Instant};

use falco_net::diagnostics::{self, LocalSetup};
use falco_net::doctor::{Check, Level};
use falco_net::{NetStatus, PeerPath};
use falco_proto::Identity;
use serde_json::json;

use crate::core::Shared;
use crate::social::{self, FriendLiveState};

fn check(id: &'static str, level: Level) -> Check {
    Check { id, level, value: None, action: None }
}

fn with(mut c: Check, value: Option<String>, action: Option<&'static str>) -> Check {
    c.value = value;
    c.action = action;
    c
}

/// Latenza misurata nell'ultima sessione da ospite (statistiche di Moonlight).
pub fn last_session_check(raw: Option<&str>) -> Check {
    let Some(v) = raw.and_then(|r| serde_json::from_str::<serde_json::Value>(r).ok()) else {
        return with(check("lastSession", Level::Pending), None, Some("playOnce"));
    };
    let st = &v["stats"];
    let total = v["latencyMs"].as_f64().unwrap_or(0.0);
    let host = st["hostLatencyMs"].as_f64().unwrap_or(0.0);
    let net = st["networkLatencyMs"].as_f64().unwrap_or(0.0);
    let dropped = st["droppedNetworkPct"].as_f64().unwrap_or(0.0)
        + st["droppedJitterPct"].as_f64().unwrap_or(0.0);
    let value = Some(format!("{total:.1} ms · {dropped:.1}%"));
    if dropped > 2.0 {
        with(check("lastSession", Level::Warn), value, Some("framesDropped"))
    } else if net > 30.0 {
        with(check("lastSession", Level::Warn), value, Some("networkLatency"))
    } else if host > 12.0 {
        with(check("lastSession", Level::Warn), value, Some("hostLatency"))
    } else if total > 25.0 {
        with(check("lastSession", Level::Warn), value, Some("clientLatency"))
    } else {
        with(check("lastSession", Level::Ok), value, None)
    }
}

/// Controlli sulla postazione locale.
pub fn local_checks(l: &LocalSetup) -> Vec<Check> {
    let mut out = vec![if l.wifi {
        match l.wifi_band.as_deref() {
            Some("2.4") => {
                with(check("wifi", Level::Warn), Some("2.4 GHz".into()), Some("useCable"))
            }
            Some(b) => {
                with(check("wifi", Level::Ok), Some(format!("{b} GHz")), Some("cableBetter"))
            }
            None => with(check("wifi", Level::Warn), Some("Wi-Fi".into()), Some("cableBetter")),
        }
    } else {
        with(check("wifi", Level::Ok), Some("Ethernet".into()), None)
    }];
    if l.on_battery {
        out.push(with(check("battery", Level::Warn), None, Some("plugIn")));
    }
    if l.power_saver {
        out.push(with(check("powerSaver", Level::Warn), None, Some("highPerformance")));
    }
    if !l.other_vpns.is_empty() {
        out.push(with(
            check("otherVpn", Level::Warn),
            Some(l.other_vpns.join(", ")),
            Some("disableVpn"),
        ));
    }
    out
}

fn netcheck_checks(n: Option<diagnostics::Netcheck>) -> Vec<Check> {
    let Some(n) = n else {
        return vec![with(check("udp", Level::Fail), None, Some("startTailscale"))];
    };
    let mut out = vec![if n.udp {
        with(
            check("udp", Level::Ok),
            Some(format!("IPv4 {} · IPv6 {}", yes(n.ipv4), yes(n.ipv6))),
            None,
        )
    } else {
        with(check("udp", Level::Fail), None, Some("unblockUdp"))
    }];
    let mapper = n.upnp == Some(true) || n.pmp == Some(true) || n.pcp == Some(true);
    out.push(match (n.mapping_varies_by_dest_ip, mapper) {
        (Some(true), false) => with(check("nat", Level::Warn), None, Some("enableUpnp")),
        (Some(true), true) => with(check("nat", Level::Ok), Some("UPnP/NAT-PMP".into()), None),
        _ => check("nat", Level::Ok),
    });
    out
}

fn yes(b: bool) -> &'static str {
    if b { "✓" } else { "✗" }
}

/// Porte di Sunshine raggiungibili sull'indirizzo Tailscale (firewall).
async fn port_checks(hosting: bool, base: u16, ip: Option<IpAddr>) -> Check {
    if !hosting {
        return with(check("sunshinePorts", Level::Pending), None, Some("hostOff"));
    }
    let Some(ip) = ip else { return check("sunshinePorts", Level::Pending) };
    let ports = [base - 5, base, base + 21];
    let mut closed = Vec::new();
    for p in ports {
        let ok =
            tokio::time::timeout(Duration::from_secs(2), tokio::net::TcpStream::connect((ip, p)))
                .await
                .is_ok_and(|r| r.is_ok());
        if !ok {
            closed.push(p.to_string());
        }
    }
    if closed.is_empty() {
        with(
            check("sunshinePorts", Level::Ok),
            Some(format!("{}, {}, {}", ports[0], ports[1], ports[2])),
            None,
        )
    } else {
        with(check("sunshinePorts", Level::Fail), Some(closed.join(", ")), Some("firewall"))
    }
}

/// Banda: 3 s di download dal server di presenza di un amico.
fn bandwidth(me: &Identity, addr: &str) -> Option<f64> {
    let t0 = Instant::now();
    let (status, _, body) =
        social::fpp_call(me, "GET", addr, "/v1/speedtest", None, None, Duration::from_secs(8))
            .ok()?;
    let secs = t0.elapsed().as_secs_f64();
    (status == 200 && secs > 0.0).then(|| body.len() as f64 * 8.0 / secs / 1_000_000.0)
}

/// Jitter e perdite: 200 pacchetti UDP a 5 ms verso l'eco dell'amico.
async fn jitter(addr: &str) -> Option<(f64, f64)> {
    let (ip, port) = social::split_address(addr);
    let target: SocketAddr = (ip.parse::<IpAddr>().ok()?, port).into();
    let bind: SocketAddr = if target.is_ipv4() { "0.0.0.0:0" } else { "[::]:0" }.parse().ok()?;
    let sock = tokio::net::UdpSocket::bind(bind).await.ok()?;
    sock.connect(target).await.ok()?;
    let start = Instant::now();
    let mut sent = [Duration::ZERO; 200];
    let mut rtts = Vec::with_capacity(200);
    let mut buf = [0u8; 64];
    let mut i = 0;
    while i < sent.len() {
        sent[i] = start.elapsed();
        let mut pkt = b"FALCOECHO".to_vec();
        pkt.extend_from_slice(&(i as u32).to_be_bytes());
        let _ = sock.send(&pkt).await;
        // Raccoglie le risposte arrivate nel frattempo, senza bloccare l'invio
        while let Ok(Ok(n)) =
            tokio::time::timeout(Duration::from_millis(5), sock.recv(&mut buf)).await
        {
            collect(&buf[..n], &sent, start, &mut rtts);
        }
        i += 1;
    }
    while let Ok(Ok(n)) =
        tokio::time::timeout(Duration::from_millis(500), sock.recv(&mut buf)).await
    {
        collect(&buf[..n], &sent, start, &mut rtts);
    }
    if rtts.is_empty() {
        return None;
    }
    let loss = 100.0 * (200 - rtts.len()) as f64 / 200.0;
    let jit =
        rtts.windows(2).map(|w| (w[1] - w[0]).abs()).sum::<f64>() / (rtts.len().max(2) - 1) as f64;
    Some((jit, loss))
}

fn collect(pkt: &[u8], sent: &[Duration; 200], start: Instant, rtts: &mut Vec<f64>) {
    if pkt.len() == 13 && pkt.starts_with(b"FALCOECHO") {
        let i = u32::from_be_bytes([pkt[9], pkt[10], pkt[11], pkt[12]]) as usize;
        if let Some(t) = sent.get(i) {
            rtts.push((start.elapsed() - *t).as_secs_f64() * 1000.0);
        }
    }
}

/// Esegue tutti i controlli. Il lock del core si tiene solo per brevi letture.
pub async fn run(shared: &Shared) -> Vec<Check> {
    let (ts, warn_exit, me, friends) = {
        let c = shared.lock().await;
        let friends: Vec<(String, Option<String>, bool)> = social::friend_views(&c)
            .into_iter()
            .map(|f| {
                (
                    f.name,
                    c.live.get(&f.id).and_then(|l| l.address.clone()),
                    f.state != FriendLiveState::Offline,
                )
            })
            .collect();
        (
            c.net.clone(),
            c.config.config().network.warn_exit_node,
            c.me.as_ref().map(|m| m.identity.clone()),
            friends,
        )
    };
    let net: NetStatus = crate::core::Core::net_status(ts.clone()).await;
    let mut out: Vec<Check> = falco_net::doctor::tailscale_checks(&net, warn_exit)
        .into_iter()
        .filter(|c| c.level != Level::Pending && c.id != "peersRelay")
        .collect();
    out.extend(netcheck_checks(diagnostics::netcheck(&ts).await));

    // Percorso verso ogni amico raggiungibile
    for (name, addr, online) in &friends {
        let Some(addr) = addr.as_ref().filter(|_| *online) else { continue };
        let (ip, _) = social::split_address(addr);
        if let Some(p) = diagnostics::ping(&ts, &ip).await {
            let value = format!("{name} · {:.0} ms", p.rtt_ms);
            out.push(if p.direct {
                with(check("peerPath", Level::Ok), Some(value), None)
            } else {
                with(
                    check("peerPath", Level::Warn),
                    Some(format!("{value} · relay {}", p.relay.unwrap_or_default())),
                    Some("enableUpnp"),
                )
            });
        } else if net.self_ips.iter().any(|s| s.to_string() == ip) {
            // Stesso PC (prove locali): nessun percorso da misurare
        } else {
            out.push(with(
                check("peerPath", Level::Fail),
                Some(name.clone()),
                Some("friendOffline"),
            ));
        }
    }
    let relayed = net.peers.iter().filter(|p| matches!(p.path, PeerPath::Relay { .. })).count();
    if relayed > 0 && !out.iter().any(|c| c.id == "peerPath") {
        out.push(with(
            check("peersRelay", Level::Warn),
            Some(relayed.to_string()),
            Some("enableUpnp"),
        ));
    }

    let (hosting, base) = {
        let c = shared.lock().await;
        (c.host.api().is_some(), c.config.config().network.sunshine_base_port)
    };
    let ip = net.self_ips.iter().find(|i| i.is_ipv4()).copied();
    out.push(port_checks(hosting, base, ip).await);

    // Banda e stabilità verso il primo amico raggiungibile
    let target = friends
        .iter()
        .find(|(_, a, on)| a.is_some() && *on)
        .and_then(|(n, a, _)| a.clone().map(|a| (n.clone(), a)));
    match (me, target) {
        (Some(me), Some((name, addr))) => {
            let a2 = addr.clone();
            let mbps =
                tokio::task::spawn_blocking(move || bandwidth(&me, &a2)).await.ok().flatten();
            out.push(match mbps {
                Some(m) if m >= 30.0 => {
                    with(check("bandwidth", Level::Ok), Some(format!("{name} · {m:.0} Mbps")), None)
                }
                Some(m) => with(
                    check("bandwidth", Level::Warn),
                    Some(format!("{name} · {m:.0} Mbps")),
                    Some("lowBandwidth"),
                ),
                None => {
                    with(check("bandwidth", Level::Fail), Some(name.clone()), Some("friendOffline"))
                }
            });
            out.push(match jitter(&addr).await {
                Some((j, l)) if l <= 1.0 && j <= 5.0 => {
                    with(check("jitter", Level::Ok), Some(format!("{j:.1} ms · {l:.0}%")), None)
                }
                Some((j, l)) => with(
                    check("jitter", Level::Warn),
                    Some(format!("{j:.1} ms · {l:.0}%")),
                    Some("unstableLine"),
                ),
                None => with(check("jitter", Level::Fail), None, Some("unblockUdp")),
            });
        }
        _ => {
            out.push(with(check("bandwidth", Level::Pending), None, Some("needFriend")));
            out.push(with(check("jitter", Level::Pending), None, Some("needFriend")));
        }
    }

    out.extend(local_checks(&diagnostics::local_setup().await));
    let last = shared.lock().await.db.meta("last_session").ok().flatten();
    out.push(last_session_check(last.as_deref()));
    out
}

/// Risultato pronto per l'IPC.
pub fn to_json(checks: &[Check]) -> serde_json::Value {
    json!({ "checks": checks })
}

/// Profilo di condivisione consigliato dai controlli: `(profilo, motivi)`.
/// La latenza resta la priorità; si scende solo quando la rete lo impone.
pub fn recommend(checks: &[Check]) -> (&'static str, Vec<&'static str>) {
    let warn = |id: &str| {
        checks.iter().any(|c| c.id == id && c.level != Level::Ok && c.level != Level::Pending)
    };
    let mbps = checks
        .iter()
        .filter(|c| c.id == "bandwidth")
        .filter_map(|c| {
            c.value.as_deref()?.rsplit(" · ").next()?.trim_end_matches(" Mbps").parse::<f64>().ok()
        })
        .reduce(f64::min);
    let mut reasons = Vec::new();
    let relay = warn("peerPath") || warn("peersRelay");
    if relay {
        reasons.push("relay");
    }
    if mbps.is_some_and(|m| m < 20.0) {
        reasons.push("lowBandwidth");
    }
    if relay || !reasons.is_empty() {
        return ("slow", reasons);
    }
    if mbps.is_some_and(|m| m < 40.0) {
        reasons.push("mediumBandwidth");
    }
    if warn("jitter") {
        reasons.push("jitter");
    }
    if warn("wifi") {
        reasons.push("wifi");
    }
    if checks.iter().any(|c| c.id == "lastSession" && c.action == Some("framesDropped")) {
        reasons.push("framesDropped");
    }
    if !reasons.is_empty() {
        return ("balanced", reasons);
    }
    if mbps.is_none() {
        reasons.push("noFriend");
    } else {
        reasons.push("fastNetwork");
    }
    ("latency", reasons)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recommendation_follows_the_network() {
        let c = |id: &'static str, level, value: Option<&str>| Check {
            id,
            level,
            value: value.map(str::to_owned),
            action: None,
        };
        assert_eq!(recommend(&[]).0, "latency");
        let slow = [c("bandwidth", Level::Warn, Some("Luca · 12 Mbps"))];
        assert_eq!(recommend(&slow), ("slow", vec!["lowBandwidth"]));
        let mid = [c("bandwidth", Level::Ok, Some("Luca · 35 Mbps")), c("wifi", Level::Warn, None)];
        assert_eq!(recommend(&mid), ("balanced", vec!["mediumBandwidth", "wifi"]));
        let fast = [c("bandwidth", Level::Ok, Some("Luca · 300 Mbps"))];
        assert_eq!(recommend(&fast), ("latency", vec!["fastNetwork"]));
    }

    #[test]
    fn last_session_thresholds() {
        let mk = |net: f64, host: f64, drop: f64| {
            format!(
                r#"{{"latencyMs":{},"stats":{{"hostLatencyMs":{host},"networkLatencyMs":{net},"droppedNetworkPct":{drop},"droppedJitterPct":0}}}}"#,
                net + host + 1.0
            )
        };
        assert_eq!(last_session_check(Some(&mk(1.0, 3.9, 0.2))).level, Level::Ok);
        assert_eq!(last_session_check(Some(&mk(40.0, 3.9, 0.2))).action, Some("networkLatency"));
        assert_eq!(last_session_check(Some(&mk(5.0, 15.0, 0.2))).action, Some("hostLatency"));
        assert_eq!(last_session_check(Some(&mk(5.0, 3.0, 5.0))).action, Some("framesDropped"));
        assert_eq!(last_session_check(None).level, Level::Pending);
    }

    #[test]
    fn local_warnings() {
        let l = LocalSetup {
            wifi: true,
            wifi_band: Some("2.4".into()),
            on_battery: true,
            power_saver: false,
            other_vpns: vec!["Radmin VPN".into()],
        };
        let c = local_checks(&l);
        assert_eq!(c.iter().filter(|c| c.level == Level::Warn).count(), 3);
        assert!(c.iter().any(|c| c.id == "otherVpn" && c.value.as_deref() == Some("Radmin VPN")));
    }
}
