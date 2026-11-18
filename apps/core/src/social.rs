//! Amici e presenza (§5): client FPP, stato «live» degli amici, ciclo di aggiornamento.

use std::net::IpAddr;
use std::time::Duration;

use falco_proto::fpp::{self, DEFAULT_PRESENCE_PORT, Presence};
use falco_proto::ipc::events;
use falco_proto::profile::{Device, SignedProfile};
use falco_proto::{Identity, Status};
use falco_store::db::{DeviceRow, FriendRow, FriendState};
use serde::Serialize;
use serde_json::json;

use crate::core::{Core, Shared, now};

/// Stato di un amico come lo mostra la UI (§5 «Stati di un amico»).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum FriendLiveState {
    Playing,
    Online,
    Busy,
    Offline,
    Pending,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Live {
    pub presence: Option<Presence>,
    pub etag: Option<String>,
    /// Indirizzo (ip:porta di presenza) che ha risposto.
    pub address: Option<String>,
}

/// Indirizzo di presenza salvato come `ip` o `ip:porta`.
pub fn split_address(s: &str) -> (String, u16) {
    match s.rsplit_once(':') {
        Some((ip, port)) if !ip.contains(':') || ip.ends_with(']') => {
            (ip.trim_matches(['[', ']']).to_owned(), port.parse().unwrap_or(DEFAULT_PRESENCE_PORT))
        }
        _ => (s.to_owned(), DEFAULT_PRESENCE_PORT),
    }
}

pub fn join_address(ip: &str, port: u16) -> String {
    let ip = if ip.contains(':') { format!("[{ip}]") } else { ip.to_owned() };
    if port == DEFAULT_PRESENCE_PORT { ip } else { format!("{ip}:{port}") }
}

/// Richiesta firmata verso un amico. Bloccante: va eseguita con `spawn_blocking`.
pub fn fpp_call(
    me: &Identity,
    method: &str,
    address: &str,
    path: &str,
    body: Option<&serde_json::Value>,
    etag: Option<&str>,
    timeout: Duration,
) -> anyhow::Result<(u16, Option<String>, Vec<u8>)> {
    let (ip, port) = split_address(address);
    let host = if ip.contains(':') { format!("[{ip}]") } else { ip };
    let url = format!("http://{host}:{port}{path}");
    let bytes = body.map(serde_json::to_vec).transpose()?.unwrap_or_default();
    let headers = fpp::sign_request(me, method, path, now(), &bytes);
    let agent = ureq::Agent::config_builder()
        .timeout_global(Some(timeout))
        .http_status_as_error(false)
        .build()
        .new_agent();
    let mut resp = if method == "POST" {
        let mut r = agent.post(&url).header("content-type", "application/json");
        for (k, v) in &headers {
            r = r.header(*k, v);
        }
        r.send(&bytes[..])?
    } else {
        let mut r = agent.get(&url);
        for (k, v) in &headers {
            r = r.header(*k, v);
        }
        if let Some(e) = etag {
            r = r.header("if-none-match", e);
        }
        r.call()?
    };
    let status = resp.status().as_u16();
    let tag = resp.headers().get("etag").and_then(|v| v.to_str().ok()).map(str::to_owned);
    let body = resp.body_mut().read_to_vec()?;
    Ok((status, tag, body))
}

/// Dispositivi di questo PC per il profilo.
pub fn my_devices(net: &falco_net::NetStatus) -> Vec<Device> {
    let Some(id) = net.self_node_id.clone() else { return Vec::new() };
    vec![Device {
        node_id: id,
        name: std::env::var("COMPUTERNAME").unwrap_or_else(|_| "falco".into()).to_lowercase(),
        os: if cfg!(windows) { "windows" } else { "macOS" }.into(),
        can_host: true,
    }]
}

/// Presenza che questo core espone agli amici.
pub fn my_presence(c: &Core) -> Presence {
    let cfg = c.config.config();
    let status = c.status();
    let visible = status != Status::Invisible;
    let ready = c.host.state == crate::host::HostState::Ready;
    Presence {
        status: visible.then_some(status),
        can_join: visible && ready && status == Status::Online,
        hosting: (visible && ready).then(|| fpp::Hosting {
            app: None,
            slots_free: cfg.host.max_guests.saturating_sub(c.guests.len() as u8),
            guests: c.guests.values().map(|g| g.name.clone()).collect(),
        }),
        profile_updated_at: c.me.as_ref().map(|m| m.updated_at()).unwrap_or(0),
        sunshine_port: cfg.network.sunshine_base_port,
    }
}

/// Vista di un amico per la UI.
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FriendView {
    pub id: String,
    pub tag: String,
    pub name: String,
    pub accent: String,
    pub avatar_url: Option<String>,
    pub state: FriendLiveState,
    pub can_join: bool,
    pub slots_free: Option<u8>,
    pub favorite: bool,
    pub last_seen: Option<i64>,
    pub role: Option<String>,
    pub devices: Vec<DeviceRow>,
}

fn live_state(row: &FriendRow, live: Option<&Live>) -> FriendLiveState {
    if row.state == FriendState::Pending {
        return FriendLiveState::Pending;
    }
    match live.and_then(|l| l.presence.as_ref()) {
        None => FriendLiveState::Offline,
        Some(p) => match p.status {
            None => FriendLiveState::Offline,
            Some(Status::Busy) => FriendLiveState::Busy,
            Some(_) if p.hosting.as_ref().is_some_and(|h| h.slots_free > 0) && p.can_join => {
                FriendLiveState::Playing
            }
            Some(_) => FriendLiveState::Online,
        },
    }
}

pub fn friend_views(c: &Core) -> Vec<FriendView> {
    let rows = c.db.friends().unwrap_or_default();
    let mut out: Vec<FriendView> = rows
        .iter()
        .map(|r| {
            let live = c.live.get(&r.id);
            let presence = live.and_then(|l| l.presence.as_ref());
            FriendView {
                id: r.id.clone(),
                tag: r.tag.clone(),
                name: r.name.clone(),
                accent: r.accent.clone(),
                avatar_url: r
                    .avatar_hash
                    .as_deref()
                    .and_then(|h| c.db.avatar(h).ok().flatten())
                    .map(|b| crate::identity::data_url(&b)),
                state: live_state(r, live),
                can_join: presence.is_some_and(|p| p.can_join),
                slots_free: presence.and_then(|p| p.hosting.as_ref()).map(|h| h.slots_free),
                favorite: r.favorite,
                last_seen: r.last_seen,
                role: c.db.permissions(&r.id).ok().flatten().map(|p| p.0),
                devices: c.db.devices(&r.id).unwrap_or_default(),
            }
        })
        .collect();
    // In gioco con posti liberi per primi (§5)
    out.sort_by_key(|f| match f.state {
        FriendLiveState::Playing => 0,
        FriendLiveState::Online => 1,
        FriendLiveState::Busy => 2,
        FriendLiveState::Pending => 3,
        FriendLiveState::Offline => 4,
    });
    out
}

/// Salva un profilo firmato ricevuto, se valido e più recente (§4 «Regole»).
pub fn accept_profile(c: &Core, sp: &SignedProfile) -> bool {
    let p = match sp.verify() {
        Ok(p) => p,
        Err(_) => return false,
    };
    let id = p.id.to_string();
    let cached =
        c.db.friend_profile(&id)
            .ok()
            .flatten()
            .and_then(|j| serde_json::from_str::<SignedProfile>(&j).ok());
    if cached.as_ref().is_some_and(|old| !sp.supersedes(&old.profile)) {
        return false;
    }
    let Ok(Some(mut row)) = c.db.friend(&id) else { return false };
    row.name.clone_from(&p.name);
    row.avatar_hash.clone_from(&p.avatar);
    row.accent.clone_from(&p.accent);
    row.tag.clone_from(&p.tag);
    let _ = c.db.upsert_friend(&row);
    if let Ok(json) = serde_json::to_string(sp) {
        let _ = c.db.set_friend_profile(&id, &json);
    }
    true
}

/// Aggiorna la presenza degli amici; ritorna true se qualcosa è cambiato.
async fn poll_once(shared: &Shared, favorites_only: bool) -> bool {
    let (me, targets) = {
        let c = shared.lock().await;
        let Some(me) = c.me.as_ref().map(|m| m.identity.clone()) else { return false };
        let targets: Vec<(FriendRow, Vec<String>, Option<String>)> = c
            .db
            .friends()
            .unwrap_or_default()
            .into_iter()
            .filter(|f| !favorites_only || f.favorite)
            .map(|f| {
                let mut addrs: Vec<String> =
                    c.db.devices(&f.id).unwrap_or_default().into_iter().map(|d| d.ts_ip).collect();
                // Provider LAN: stessa rete locale, latenza minima (§9)
                if let Some(p) = c.lan_peers.get(&f.id) {
                    addrs.insert(0, join_address(&p.ip.to_string(), p.port));
                }
                let etag = c.live.get(&f.id).and_then(|l| l.etag.clone());
                (f, addrs, etag)
            })
            .collect();
        (me, targets)
    };

    let mut changed = false;
    for (friend, addrs, etag) in targets {
        let me2 = me.clone();
        let result = tokio::task::spawn_blocking(move || {
            for a in addrs {
                if let Ok(r) = fpp_call(
                    &me2,
                    "GET",
                    &a,
                    "/v1/presence",
                    None,
                    etag.as_deref(),
                    Duration::from_secs(3),
                ) {
                    return Some((a, r));
                }
            }
            None
        })
        .await
        .ok()
        .flatten();

        let mut c = shared.lock().await;
        let before = c.live.get(&friend.id).cloned().unwrap_or_default();
        let mut live = before.clone();
        match result {
            Some((addr, (304, _, _))) => live.address = Some(addr),
            Some((addr, (200, tag, body))) => {
                live.presence = serde_json::from_slice(&body).ok();
                live.etag = tag;
                live.address = Some(addr);
            }
            _ => live = Live::default(),
        }
        if live.presence.is_some() {
            let _ = c.db.touch_friend(&friend.id, now());
        }
        // Profilo più recente: lo scarica e verifica la firma
        let newer = live.presence.as_ref().map(|p| p.profile_updated_at).unwrap_or(0);
        let cached_at =
            c.db.friend_profile(&friend.id)
                .ok()
                .flatten()
                .and_then(|j| serde_json::from_str::<SignedProfile>(&j).ok())
                .map(|p| p.profile.updated_at)
                .unwrap_or(0);
        let fetch = newer > cached_at;
        let address = live.address.clone();
        if live != before {
            changed = true;
        }
        c.live.insert(friend.id.clone(), live);
        drop(c);
        if fetch && let Some(addr) = address {
            changed |= refresh_profile(shared, &me, &addr).await;
        }
    }
    changed
}

/// Scarica profilo e avatar di un amico.
pub async fn refresh_profile(shared: &Shared, me: &Identity, address: &str) -> bool {
    let (me2, a) = (me.clone(), address.to_owned());
    let profile = tokio::task::spawn_blocking(move || {
        fpp_call(&me2, "GET", &a, "/v1/profile", None, None, Duration::from_secs(5)).ok()
    })
    .await
    .ok()
    .flatten()
    .and_then(|(s, _, b)| {
        (s == 200).then(|| serde_json::from_slice::<SignedProfile>(&b).ok()).flatten()
    });
    let Some(sp) = profile else { return false };
    let accepted = accept_profile(&*shared.lock().await, &sp);
    if let Some(hash) = sp.profile.avatar.clone() {
        let has = shared.lock().await.db.avatar(&hash).ok().flatten().is_some();
        if !has {
            let (me3, a3, h3) = (me.clone(), address.to_owned(), hash.clone());
            let bytes = tokio::task::spawn_blocking(move || {
                fpp_call(
                    &me3,
                    "GET",
                    &a3,
                    &format!("/v1/avatar/{h3}"),
                    None,
                    None,
                    Duration::from_secs(5),
                )
                .ok()
            })
            .await
            .ok()
            .flatten();
            if let Some((200, _, b)) = bytes {
                // Cache per hash: si accetta solo se l'hash corrisponde
                use sha2::Digest;
                if data_encoding::HEXLOWER.encode(&sha2::Sha256::digest(&b)) == hash {
                    let _ = shared.lock().await.db.put_avatar(&hash, &b);
                }
            }
        }
    }
    accepted
}

pub fn emit_friends(c: &Core) {
    c.emit(events::PRESENCE_CHANGED, json!({ "friends": friend_views(c) }));
    c.refresh_tray();
}

/// Ciclo di presenza: 15 s a finestra aperta, 60 s in tray (solo preferiti), fermo in streaming.
pub async fn run(shared: Shared) {
    let poke = shared.lock().await.poke.clone();
    loop {
        let (wait, skip, favorites_only) = {
            let c = shared.lock().await;
            let p = &c.config.config().performance;
            let ui = c.ui_clients > 0;
            let s = if ui { p.poll_ui_s } else { p.poll_tray_s };
            (Duration::from_secs(u64::from(s)), c.streaming || c.me.is_none(), !ui)
        };
        if !skip && poll_once(&shared, favorites_only).await {
            emit_friends(&*shared.lock().await);
        }
        tokio::select! {
            _ = tokio::time::sleep(wait) => {}
            _ = poke.notified() => {
                // Gli eventi arrivano a raffiche: un secondo di pausa li raggruppa
                tokio::time::sleep(Duration::from_secs(1)).await;
            }
        }
    }
}

/// IP da cui chiamare un amico: solo indirizzi Tailscale o di loopback.
pub fn is_tailnet(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => {
            let o = v4.octets();
            v4.is_loopback() || (o[0] == 100 && (64..128).contains(&o[1]))
        }
        IpAddr::V6(v6) => v6.is_loopback() || v6.segments()[..3] == [0xfd7a, 0x115c, 0xa1e0],
    }
}

/// Rimuove o blocca un amico (§5): rifiuto immediato, revoca del pairing, avviso all'amico.
/// La revoca della condivisione Tailscale resta manuale finché l'API non la consente (ADR 0009).
pub async fn remove_friend(shared: &Shared, id: &str, block: bool) {
    let (me, address, client) = {
        let mut c = shared.lock().await;
        let client = c.db.permissions(id).ok().flatten().and_then(|p| p.2);
        let address = c.live.get(id).and_then(|l| l.address.clone());
        if block {
            let _ = c.db.set_friend_state(id, FriendState::Blocked);
        } else {
            let _ = c.db.remove_friend(id);
        }
        c.live.remove(id);
        emit_friends(&c);
        (c.me.as_ref().map(|m| m.identity.clone()), address, client)
    };
    if let (Some(uuid), Some(api)) = (client, shared.lock().await.host.api()) {
        let _ = tokio::task::spawn_blocking(move || api.unpair(&uuid)).await;
    }
    if let (Some(me), Some(addr)) = (me, address) {
        let _ = tokio::task::spawn_blocking(move || {
            fpp_call(
                &me,
                "POST",
                &addr,
                "/v1/friend/remove",
                Some(&json!({})),
                None,
                Duration::from_secs(3),
            )
        })
        .await;
    }
    crate::hub::unfriend(shared, id).await;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn addresses() {
        assert_eq!(split_address("100.64.0.2"), ("100.64.0.2".into(), 47900));
        assert_eq!(split_address("100.64.0.2:47901"), ("100.64.0.2".into(), 47901));
        assert_eq!(split_address("[fd7a::1]:47901"), ("fd7a::1".into(), 47901));
        assert_eq!(join_address("100.64.0.2", 47900), "100.64.0.2");
        assert_eq!(join_address("100.64.0.2", 47901), "100.64.0.2:47901");
    }

    #[test]
    fn tailnet_ranges() {
        assert!(is_tailnet("100.70.2.107".parse().unwrap()));
        assert!(is_tailnet("fd7a:115c:a1e0::4f01:2f3".parse().unwrap()));
        assert!(!is_tailnet("192.168.1.10".parse().unwrap()));
        assert!(!is_tailnet("100.200.0.1".parse().unwrap()));
    }
}
