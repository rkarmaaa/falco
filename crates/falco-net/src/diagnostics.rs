//! Raccolta dei dati per il Dottore di rete (§9): netcheck, ping, postazione locale.

use std::time::Duration;

use serde::{Deserialize, Serialize};
use tokio::process::Command;

use crate::tailscale::Tailscale;

/// Sottoinsieme di `tailscale netcheck --format json`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct Netcheck {
    #[serde(rename = "UDP", default)]
    pub udp: bool,
    #[serde(rename = "IPv4", default)]
    pub ipv4: bool,
    #[serde(rename = "IPv6", default)]
    pub ipv6: bool,
    /// NAT «difficile»: la porta pubblica cambia per destinazione.
    #[serde(rename = "MappingVariesByDestIP", default)]
    pub mapping_varies_by_dest_ip: Option<bool>,
    #[serde(rename = "UPnP", default)]
    pub upnp: Option<bool>,
    #[serde(rename = "PMP", default)]
    pub pmp: Option<bool>,
    #[serde(rename = "PCP", default)]
    pub pcp: Option<bool>,
    #[serde(rename = "PreferredDERP", default)]
    pub preferred_derp: u32,
    #[serde(rename = "GlobalV4", default)]
    pub global_v4: Option<String>,
}

/// Estrae il JSON dall'output (le prime righe possono essere commenti o log).
pub fn parse_netcheck(out: &str) -> Option<Netcheck> {
    let start = out.find('{')?;
    serde_json::from_str(&out[start..]).ok()
}

pub async fn netcheck(ts: &Tailscale) -> Option<Netcheck> {
    let out = ts.run(&["netcheck", "--format", "json"]).await.ok()?;
    parse_netcheck(&String::from_utf8_lossy(&out))
}

/// Esito di `tailscale ping` verso un nodo.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PingResult {
    pub direct: bool,
    /// Regione DERP se via relay.
    pub relay: Option<String>,
    pub rtt_ms: f32,
}

/// `pong from luca-pc (100.64.0.2) via DERP(fra) in 45ms` · `… via 93.4.1.2:41641 in 6ms`
pub fn parse_ping(out: &str) -> Option<PingResult> {
    let line = out.lines().rev().find(|l| l.contains("pong from"))?;
    let via = line.split(" via ").nth(1)?;
    let (path, rest) = via.split_once(" in ")?;
    let rtt_ms: f32 = rest.trim().trim_end_matches("ms").trim().parse().ok()?;
    let relay = path.strip_prefix("DERP(").map(|r| r.trim_end_matches(')').to_owned());
    Some(PingResult { direct: relay.is_none(), relay, rtt_ms })
}

pub async fn ping(ts: &Tailscale, ip: &str) -> Option<PingResult> {
    let out = ts.run(&["ping", "--c", "5", "--timeout", "3s", ip]).await.ok()?;
    parse_ping(&String::from_utf8_lossy(&out))
}

/// Postazione locale: cosa peggiora lo stream (§10 «Durante lo streaming»).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalSetup {
    pub wifi: bool,
    /// `2.4`, `5` o `6` GHz se in Wi-Fi.
    pub wifi_band: Option<String>,
    pub on_battery: bool,
    pub power_saver: bool,
    /// Altre VPN attive oltre a Tailscale.
    pub other_vpns: Vec<String>,
}

const VPN_HINTS: &[&str] = &[
    "vpn",
    "wireguard",
    "openvpn",
    "tap-windows",
    "hamachi",
    "zerotier",
    "nordlynx",
    "wintun",
    "radmin",
];

/// Adattatori (nome, descrizione) che sembrano VPN, escluso Tailscale.
pub fn detect_vpns(adapters: &[(String, String)]) -> Vec<String> {
    adapters
        .iter()
        .filter(|(n, d)| {
            let s = format!("{n} {d}").to_lowercase();
            !s.contains("tailscale") && VPN_HINTS.iter().any(|h| s.contains(h))
        })
        .map(|(n, _)| n.clone())
        .collect()
}

/// Banda Wi-Fi dall'output di `netsh wlan show interfaces` (anche in italiano).
pub fn parse_wifi_band(out: &str) -> Option<String> {
    let l = out.to_lowercase();
    if l.contains("6 ghz") {
        Some("6".into())
    } else if l.contains("5 ghz") {
        Some("5".into())
    } else if l.contains("2.4 ghz") || l.contains("2,4 ghz") {
        Some("2.4".into())
    } else {
        None
    }
}

async fn run(cmd: &str, args: &[&str]) -> Option<String> {
    let mut c = Command::new(cmd);
    c.args(args).kill_on_drop(true);
    #[cfg(windows)]
    c.creation_flags(0x0800_0000);
    let out = tokio::time::timeout(Duration::from_secs(8), c.output()).await.ok()?.ok()?;
    Some(String::from_utf8_lossy(&out.stdout).into_owned())
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct Adapter {
    name: String,
    interface_description: String,
    #[serde(default)]
    physical_media_type: Option<String>,
}

#[cfg(windows)]
pub async fn local_setup() -> LocalSetup {
    let script = "@{a=@(Get-NetAdapter | Where-Object Status -eq Up | Select-Object Name,InterfaceDescription,PhysicalMediaType); \
                  b=@(Get-CimInstance Win32_Battery | Select-Object -ExpandProperty BatteryStatus); \
                  p=(powercfg /getactivescheme)} | ConvertTo-Json -Depth 3 -Compress";
    let raw = run("powershell", &["-NoProfile", "-Command", script]).await.unwrap_or_default();
    let v: serde_json::Value = serde_json::from_str(raw.trim()).unwrap_or_default();
    let adapters: Vec<Adapter> = serde_json::from_value(v["a"].clone()).unwrap_or_default();
    let wifi = adapters.iter().any(|a| {
        a.physical_media_type.as_deref().is_some_and(|m| m.contains("802.11"))
            || a.interface_description.to_lowercase().contains("wi-fi")
            || a.interface_description.to_lowercase().contains("wireless")
    });
    let band = if wifi {
        run("netsh", &["wlan", "show", "interfaces"]).await.and_then(|o| parse_wifi_band(&o))
    } else {
        None
    };
    // BatteryStatus 1 = in scarica; GUID a1841308… = piano «Risparmio di energia»
    let on_battery = v["b"].as_array().is_some_and(|b| b.iter().any(|x| x.as_i64() == Some(1)));
    let power_saver =
        v["p"].as_str().is_some_and(|p| p.contains("a1841308-3541-4fab-bc81-f71556f20b4a"));
    let pairs: Vec<(String, String)> =
        adapters.into_iter().map(|a| (a.name, a.interface_description)).collect();
    LocalSetup { wifi, wifi_band: band, on_battery, power_saver, other_vpns: detect_vpns(&pairs) }
}

#[cfg(not(windows))]
pub async fn local_setup() -> LocalSetup {
    let batt = run("pmset", &["-g", "batt"]).await.unwrap_or_default();
    let on_battery = batt.contains("'Battery Power'");
    let power_saver =
        run("pmset", &["-g"]).await.is_some_and(|o| o.contains("lowpowermode         1"));
    let ifaces = run("networksetup", &["-listallhardwareports"]).await.unwrap_or_default();
    let vpns = run("scutil", &["--nc", "list"]).await.unwrap_or_default();
    let other_vpns = vpns
        .lines()
        .filter(|l| l.contains("(Connected)") && !l.to_lowercase().contains("tailscale"))
        .map(|l| l.split('"').nth(1).unwrap_or(l).to_owned())
        .collect();
    let wifi_dev = ifaces.split("Hardware Port: Wi-Fi").nth(1).and_then(|r| {
        r.lines()
            .find(|l| l.starts_with("Device:"))
            .map(|l| l.trim_start_matches("Device:").trim().to_owned())
    });
    let wifi = match &wifi_dev {
        Some(dev) => {
            run("ipconfig", &["getifaddr", dev]).await.is_some_and(|o| !o.trim().is_empty())
        }
        None => false,
    };
    LocalSetup { wifi, wifi_band: None, on_battery, power_saver, other_vpns }
}

/// Segue gli eventi di Tailscale (LocalAPI via `debug watch-ipn`): chiama `on_change` a ogni notifica.
pub async fn watch_ipn(ts: &Tailscale, mut on_change: impl FnMut()) {
    use tokio::io::{AsyncBufReadExt, BufReader};
    let Some(bin) = crate::tailscale::locate_cli() else { return };
    let _ = ts;
    let mut cmd = Command::new(bin);
    cmd.args(["debug", "watch-ipn"]).stdout(std::process::Stdio::piped()).kill_on_drop(true);
    #[cfg(windows)]
    cmd.creation_flags(0x0800_0000);
    let Ok(mut child) = cmd.spawn() else { return };
    let Some(out) = child.stdout.take() else { return };
    let mut lines = BufReader::new(out).lines();
    while let Ok(Some(line)) = lines.next_line().await {
        // Ogni notifica è un oggetto JSON multilinea che termina con «}» a inizio riga
        if line == "}" {
            on_change();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn netcheck_with_comment_header() {
        let out = "# Warning: not stable\n{\"UDP\":true,\"IPv4\":true,\"IPv6\":false,\"MappingVariesByDestIP\":true,\"UPnP\":false,\"PMP\":true,\"PCP\":false,\"PreferredDERP\":4,\"GlobalV4\":\"93.44.1.2:41641\"}";
        let n = parse_netcheck(out).unwrap();
        assert!(n.udp && n.ipv4 && !n.ipv6);
        assert_eq!(n.mapping_varies_by_dest_ip, Some(true));
        assert_eq!(n.preferred_derp, 4);
    }

    #[test]
    fn ping_direct_and_relay() {
        let d = parse_ping("pong from luca-pc (100.64.0.2) via 93.44.12.8:41641 in 6ms\n").unwrap();
        assert!(d.direct && (d.rtt_ms - 6.0).abs() < 0.01);
        let r = parse_ping("pong from bea (100.64.0.3) via DERP(fra) in 45ms").unwrap();
        assert_eq!(r.relay.as_deref(), Some("fra"));
        assert!(!r.direct);
        assert!(parse_ping("100.70.2.107 is local Tailscale IP").is_none());
    }

    #[test]
    fn vpn_and_wifi_detection() {
        let adapters = vec![
            ("Ethernet".into(), "Intel I211".into()),
            ("Radmin VPN".into(), "Famatech Radmin VPN Ethernet Adapter".into()),
            ("Tailscale".into(), "Tailscale Tunnel".into()),
        ];
        assert_eq!(detect_vpns(&adapters), ["Radmin VPN"]);
        assert_eq!(parse_wifi_band("    Banda                  : 2,4 GHz").as_deref(), Some("2.4"));
        assert_eq!(parse_wifi_band("Band : 5 GHz").as_deref(), Some("5"));
    }
}
