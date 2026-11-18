//! Provider di rete (§9): Tailscale (default), LAN, Diretto, Headscale.
//! Headscale usa lo stesso client di Tailscale con un server di controllo diverso.

use std::net::IpAddr;
use std::time::Duration;

use falco_net::NetStatus;
use falco_net::lan::{self, Lan};
use falco_store::config::{FirewallScope, Provider};
use mdns_sd::ServiceEvent;

use crate::core::Shared;
use crate::social;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Tailnet,
    Lan,
    Direct,
}

pub async fn mode(core: &Shared) -> Mode {
    let c = core.lock().await;
    let n = &c.config.config().network;
    match n.provider {
        Provider::Direct => Mode::Direct,
        Provider::Lan => Mode::Lan,
        _ if n.firewall_scope == FirewallScope::TailnetLan => Mode::Lan,
        _ => Mode::Tailnet,
    }
}

/// Indirizzi su cui ascolta il server di presenza.
pub async fn bind_ips(core: &Shared, net: &NetStatus) -> Vec<IpAddr> {
    let tailnet = net.self_ips.iter().copied().filter(|ip| social::is_tailnet(*ip));
    match mode(core).await {
        Mode::Tailnet => tailnet.collect(),
        Mode::Lan => tailnet.chain(lan::lan_ips()).collect(),
        // Consenso esplicito del provider Diretto: tutte le interfacce, firma sempre richiesta
        Mode::Direct => vec![IpAddr::from([0u8, 0, 0, 0])],
    }
}

/// Fuori dalla tailnet: l'IP sorgente deve essere quello annunciato via mDNS o registrato.
pub async fn trusted_off_tailnet(core: &Shared, friend_id: &str, ip: IpAddr) -> bool {
    if social::is_tailnet(ip) {
        return false;
    }
    let m = mode(core).await;
    let c = core.lock().await;
    match m {
        Mode::Lan => c.lan_peers.get(friend_id).is_some_and(|p| p.ip == ip),
        Mode::Direct => {
            c.db.devices(friend_id)
                .unwrap_or_default()
                .iter()
                .any(|d| social::split_address(&d.ts_ip).0 == ip.to_string())
        }
        Mode::Tailnet => false,
    }
}

/// IP pubblico da mettere negli inviti con il provider Diretto (da `tailscale netcheck`).
pub async fn public_ip(core: &Shared) -> Option<String> {
    if mode(core).await != Mode::Direct {
        return None;
    }
    let ts = core.lock().await.net.clone();
    let n = falco_net::diagnostics::netcheck(&ts).await?;
    n.global_v4.and_then(|g| g.rsplit_once(':').map(|(ip, _)| ip.to_owned()))
}

/// Annuncio e scoperta mDNS mentre il provider LAN è attivo.
pub async fn lan_task(core: Shared) {
    loop {
        let start = {
            let c = core.lock().await;
            let me = c.me.as_ref().map(|m| (m.identity.id().to_string(), m.name().to_owned()));
            me.map(|(id, name)| (id, name, c.config.config().network.presence_port))
        };
        if mode(&core).await == Mode::Lan
            && let Some((id, name, port)) = start
            && let Ok((lan, rx)) = Lan::start(&id, &name, port)
        {
            let deadline = tokio::time::Instant::now() + Duration::from_secs(300);
            while let Ok(Ok(ev)) = tokio::time::timeout_at(deadline, rx.recv_async()).await {
                if let ServiceEvent::ServiceResolved(info) = ev
                    && let Some(p) = lan::peer_from(&info)
                    && p.id != id
                {
                    let mut c = core.lock().await;
                    if c.lan_peers.get(&p.id) != Some(&p) {
                        c.lan_peers.insert(p.id.clone(), p);
                        c.poke.notify_one();
                    }
                }
            }
            // Ogni 5 minuti si riannuncia (indirizzi cambiati, provider spento)
            lan.stop();
            core.lock().await.lan_peers.clear();
        } else {
            tokio::time::sleep(Duration::from_secs(30)).await;
        }
    }
}
