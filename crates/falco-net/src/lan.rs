//! Provider LAN (§9): annuncio e scoperta di Falco nella rete locale via mDNS (`_falco._tcp`).

use std::net::{IpAddr, Ipv4Addr};

use mdns_sd::{ServiceDaemon, ServiceEvent, ServiceInfo};
use serde::Serialize;

pub const SERVICE: &str = "_falco._tcp.local.";

/// Un altro Falco visto nella rete locale.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LanPeer {
    pub id: String,
    pub name: String,
    pub ip: IpAddr,
    pub port: u16,
}

/// Indirizzi privati IPv4 (10/8, 172.16/12, 192.168/16) delle interfacce attive.
pub fn lan_ips() -> Vec<IpAddr> {
    if_addrs::get_if_addrs()
        .unwrap_or_default()
        .into_iter()
        .filter(|i| !i.is_loopback())
        .filter_map(|i| match i.ip() {
            IpAddr::V4(v4) if is_private(v4) => Some(IpAddr::V4(v4)),
            _ => None,
        })
        .collect()
}

pub fn is_private(v4: Ipv4Addr) -> bool {
    v4.is_private() && !v4.is_link_local()
}

pub struct Lan {
    daemon: ServiceDaemon,
    fullname: String,
}

impl Lan {
    /// Annuncia questo Falco e inizia a cercare gli altri.
    pub fn start(
        id: &str,
        name: &str,
        port: u16,
    ) -> Result<(Self, mdns_sd::Receiver<ServiceEvent>), mdns_sd::Error> {
        let daemon = ServiceDaemon::new()?;
        let ips = lan_ips();
        let instance: String = id.chars().take(16).collect();
        let info = ServiceInfo::new(
            SERVICE,
            &instance,
            &format!("falco-{instance}.local."),
            &ips[..],
            port,
            &[("id", id), ("name", name)][..],
        )?;
        let fullname = info.get_fullname().to_owned();
        daemon.register(info)?;
        let rx = daemon.browse(SERVICE)?;
        Ok((Self { daemon, fullname }, rx))
    }

    pub fn stop(self) {
        let _ = self.daemon.unregister(&self.fullname);
        let _ = self.daemon.shutdown();
    }
}

/// Converte un servizio risolto in un peer (primo indirizzo privato).
pub fn peer_from(info: &ServiceInfo) -> Option<LanPeer> {
    let id = info.get_property_val_str("id")?.to_owned();
    let name = info.get_property_val_str("name").unwrap_or_default().to_owned();
    let ip =
        info.get_addresses_v4().into_iter().find(|a| is_private(**a)).map(|a| IpAddr::V4(*a))?;
    Some(LanPeer { id, name, ip, port: info.get_port() })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn private_ranges() {
        assert!(is_private("192.168.1.57".parse().unwrap()));
        assert!(is_private("10.0.0.2".parse().unwrap()));
        assert!(!is_private("26.50.107.72".parse().unwrap()));
        assert!(!is_private("100.70.2.107".parse().unwrap()));
    }
}
