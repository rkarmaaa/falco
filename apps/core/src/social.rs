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
    /// Andata e ritorno dell'ultima richiesta di presenza, in ms: la latenza verso l'amico.
    pub rtt_ms: Option<u32>,
    /// Ultimo errore della presenza (stato HTTP, 0 se nessuna risposta): si registra una volta.
    pub error: Option<u16>,
    /// Ultimo invio del nostro nodo nuovo dopo un 403 (secondi Unix, ADR 0018).
    pub device_sent_at: Option<i64>,
    /// Giri di fila senza una risposta valida.
    pub misses: u8,
}

/// Ogni quanto riprovare ad annunciare il nodo nuovo: Tailscale segna offline il vecchio in
/// qualche minuto.
const DEVICE_RETRY_S: i64 = 300;

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
    /// Latenza stimata verso l'amico (presenza), se raggiungibile.
    pub rtt_ms: Option<u32>,
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
                rtt_ms: c.live.get(&r.id).and_then(|l| l.rtt_ms),
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

/// Attesa massima di una risposta alla presenza.
const PRESENCE_TIMEOUT: Duration = Duration::from_secs(3);
/// Giri di fila senza risposta prima di segnare un amico offline: un pacchetto perso non conta.
const MISSES_OFFLINE: u8 = 2;

/// Amico da interrogare: indirizzi in ordine di preferenza (LAN prima, poi Tailscale).
#[derive(Debug, Clone)]
struct Target {
    id: String,
    name: String,
    addrs: Vec<String>,
    etag: Option<String>,
}

/// Risposta HTTP di un indirizzo alla presenza.
#[derive(Debug, Clone, PartialEq)]
struct Probe {
    addr: String,
    status: u16,
    etag: Option<String>,
    body: Vec<u8>,
    rtt_ms: u32,
}

impl Probe {
    fn ok(&self) -> bool {
        matches!(self.status, 200 | 304)
    }

    /// Motivo leggibile di una risposta d'errore, per il log.
    fn reason(&self) -> String {
        let body = String::from_utf8_lossy(&self.body[..self.body.len().min(200)]).into_owned();
        format!("{} ha risposto {}: {body}", self.addr, self.status)
    }
}

fn target(c: &Core, f: &FriendRow) -> Target {
    let mut addrs: Vec<String> = Vec::new();
    // Provider LAN: stessa rete locale, latenza minima (§9)
    if let Some(p) = c.lan_peers.get(&f.id) {
        addrs.push(join_address(&p.ip.to_string(), p.port));
    }
    // L'ultimo indirizzo che ha risposto prima degli altri dispositivi registrati
    if let Some(a) = c.live.get(&f.id).and_then(|l| l.address.clone()) {
        addrs.push(a);
    }
    addrs.extend(c.db.devices(&f.id).unwrap_or_default().into_iter().map(|d| d.ts_ip));
    let mut seen = std::collections::HashSet::new();
    addrs.retain(|a| seen.insert(a.clone()));
    Target {
        id: f.id.clone(),
        name: f.name.clone(),
        addrs,
        etag: c.live.get(&f.id).and_then(|l| l.etag.clone()),
    }
}

/// Interroga gli indirizzi in ordine e si ferma al primo che risponde 200 o 304. Se nessuno
/// risponde bene, tiene il primo errore HTTP (un 403 dice più di un timeout). Bloccante.
fn probe(me: &Identity, t: &Target) -> Option<Probe> {
    let mut failed: Option<Probe> = None;
    for addr in &t.addrs {
        let started = std::time::Instant::now();
        let etag = t.etag.as_deref();
        let Ok((status, tag, body)) =
            fpp_call(me, "GET", addr, "/v1/presence", None, etag, PRESENCE_TIMEOUT)
        else {
            continue;
        };
        let rtt_ms = u32::try_from(started.elapsed().as_millis()).unwrap_or(u32::MAX);
        let p = Probe { addr: addr.clone(), status, etag: tag, body, rtt_ms };
        if p.ok() {
            return Some(p);
        }
        failed.get_or_insert(p);
    }
    failed
}

/// Nuovo stato di un amico dopo un'interrogazione. Funzione pura: tutta la logica sta qui.
fn next_live(before: &Live, probe: Option<&Probe>) -> Live {
    match probe {
        Some(p) if p.status == 304 && before.presence.is_some() => Live {
            address: Some(p.addr.clone()),
            rtt_ms: Some(p.rtt_ms),
            error: None,
            misses: 0,
            ..before.clone()
        },
        Some(p) if p.status == 200 => match serde_json::from_slice::<Presence>(&p.body) {
            Ok(presence) => Live {
                presence: Some(presence),
                etag: p.etag.clone(),
                address: Some(p.addr.clone()),
                rtt_ms: Some(p.rtt_ms),
                error: None,
                misses: 0,
                device_sent_at: before.device_sent_at,
            },
            // Risposta illeggibile (versione incompatibile?): come un errore, senza etag
            Err(_) => Live { error: Some(p.status), ..offline(before) },
        },
        // 304 senza presenza in memoria (etag vecchio): al prossimo giro si chiede tutto
        Some(p) if p.status == 304 => Live { error: None, ..offline(before) },
        Some(p) => Live { error: Some(p.status), ..offline(before) },
        // Nessuna risposta: si aspetta un altro giro prima di dirlo offline
        None if before.presence.is_some() && before.misses + 1 < MISSES_OFFLINE => {
            Live { misses: before.misses + 1, rtt_ms: None, ..before.clone() }
        }
        None => Live { error: Some(0), ..offline(before) },
    }
}

/// Stato offline che conserva solo ciò che serve tra un giro e l'altro.
fn offline(before: &Live) -> Live {
    Live {
        error: before.error,
        misses: before.misses.saturating_add(1),
        device_sent_at: before.device_sent_at,
        ..Live::default()
    }
}

/// Va annunciato il nostro nodo nuovo? Solo su 403 e al massimo ogni `DEVICE_RETRY_S`.
fn should_announce(before: &Live, probe: Option<&Probe>, now: i64) -> bool {
    probe.is_some_and(|p| p.status == 403)
        && before.device_sent_at.is_none_or(|t| now - t >= DEVICE_RETRY_S)
}

/// Interroga un amico e aggiorna il suo stato. Ritorna true se qualcosa è cambiato.
async fn update_friend(shared: &Shared, me: &Identity, t: Target) -> bool {
    let (me2, t2) = (me.clone(), t.clone());
    let result = tokio::task::spawn_blocking(move || probe(&me2, &t2)).await.ok().flatten();
    apply_probe(shared, me, &t, result).await
}

async fn apply_probe(shared: &Shared, me: &Identity, t: &Target, result: Option<Probe>) -> bool {
    let (changed, fetch, announce) = {
        let mut c = shared.lock().await;
        let before = c.live.get(&t.id).cloned().unwrap_or_default();
        let mut live = next_live(&before, result.as_ref());
        // Un errore si registra una volta, quando cambia, non a ogni giro
        if live.error.is_some() && live.error != before.error {
            let why = result.as_ref().map_or_else(|| "nessuna risposta".into(), Probe::reason);
            tracing::warn!("presenza di {}: {why}", t.name);
        }
        let announce = should_announce(&before, result.as_ref(), now())
            .then(|| result.as_ref().map(|p| p.addr.clone()))
            .flatten();
        if announce.is_some() {
            live.device_sent_at = Some(now());
        }
        if live.presence.is_some() {
            let _ = c.db.touch_friend(&t.id, now());
        }
        let fetch = live.address.clone().filter(|_| profile_outdated(&c, &t.id, &live));
        let changed = live != before;
        c.live.insert(t.id.clone(), live);
        (changed, fetch, announce)
    };
    let mut changed = changed;
    if let Some(addr) = fetch {
        changed |= refresh_profile(shared, me, &addr).await;
    }
    if let Some(addr) = announce
        && announce_device(shared, me, &addr).await
    {
        tracing::warn!("nodo nuovo registrato da {}", t.name);
        // Ora l'amico ci riconosce: nuovo giro subito, senza aspettare l'intervallo
        shared.lock().await.poke.notify_one();
        changed = true;
    }
    changed
}

/// Serve scaricare il profilo firmato? Più recente di quello in cache, o foto ancora mancante
/// (anche con il profilo già salvato dall'handshake).
fn profile_outdated(c: &Core, id: &str, live: &Live) -> bool {
    let Some(presence) = &live.presence else { return false };
    let cached =
        c.db.friend_profile(id)
            .ok()
            .flatten()
            .and_then(|j| serde_json::from_str::<SignedProfile>(&j).ok());
    let cached_at = cached.as_ref().map_or(0, |p| p.profile.updated_at);
    let avatar_missing = cached
        .as_ref()
        .and_then(|p| p.profile.avatar.as_deref())
        .is_some_and(|h| c.db.avatar(h).ok().flatten().is_none());
    presence.profile_updated_at > cached_at || avatar_missing
}

/// Aggiorna la presenza degli amici, tutti in parallelo; ritorna true se qualcosa è cambiato.
async fn poll_once(shared: &Shared, favorites_only: bool) -> bool {
    let (me, targets) = {
        let c = shared.lock().await;
        let Some(me) = c.me.as_ref().map(|m| m.identity.clone()) else { return false };
        let targets: Vec<Target> =
            c.db.friends()
                .unwrap_or_default()
                .iter()
                .filter(|f| !favorites_only || f.favorite)
                .map(|f| target(&c, f))
                .collect();
        (me, targets)
    };
    let mut probes = tokio::task::JoinSet::new();
    for t in targets {
        let me = me.clone();
        probes.spawn_blocking(move || {
            let r = probe(&me, &t);
            (t, r)
        });
    }
    let mut changed = false;
    while let Some(done) = probes.join_next().await {
        if let Ok((t, r)) = done {
            changed |= apply_probe(shared, &me, &t, r).await;
        }
    }
    changed
}

/// Indirizzo di presenza raggiungibile di un amico. Se non è noto (Falco nel tray interroga solo
/// i preferiti, o l'ultimo giro è andato a vuoto) lo chiede subito invece di arrendersi.
pub async fn reach(shared: &Shared, id: &str) -> Option<String> {
    let (me, t) = {
        let c = shared.lock().await;
        if let Some(a) =
            c.live.get(id).filter(|l| l.presence.is_some()).and_then(|l| l.address.clone())
        {
            return Some(a);
        }
        let me = c.me.as_ref()?.identity.clone();
        let f = c.db.friend(id).ok().flatten()?;
        (me, target(&c, &f))
    };
    if update_friend(shared, &me, t).await {
        emit_friends(&*shared.lock().await);
    }
    let c = shared.lock().await;
    c.live.get(id).filter(|l| l.presence.is_some()).and_then(|l| l.address.clone())
}

/// Annuncia a un amico il nodo Tailscale attuale di questo PC (ADR 0018).
async fn announce_device(shared: &Shared, me: &Identity, address: &str) -> bool {
    let ts = shared.lock().await.net.clone();
    let net = crate::core::Core::net_status(ts).await;
    let body = {
        let c = shared.lock().await;
        let Some(Ok(profile)) = c.me.as_ref().map(|m| m.signed(c.status(), my_devices(&net)))
        else {
            return false;
        };
        let port = c.config.config().network.presence_port;
        match serde_json::to_value(fpp::DeviceUpdate { profile, presence_port: port }) {
            Ok(v) => v,
            Err(_) => return false,
        }
    };
    let (me2, a) = (me.clone(), address.to_owned());
    let out = tokio::task::spawn_blocking(move || {
        fpp_call(&me2, "POST", &a, "/v1/devices", Some(&body), None, Duration::from_secs(5))
    })
    .await;
    match out {
        Ok(Ok((204, _, _))) => true,
        Ok(Ok((status, _, body))) => {
            let why = String::from_utf8_lossy(&body[..body.len().min(200)]).into_owned();
            tracing::warn!("nodo nuovo non accettato da {address} ({status}): {why}");
            false
        }
        _ => false,
    }
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
    fn presence_body() -> Vec<u8> {
        br#"{"status":"online","canJoin":true,"hosting":null,"profileUpdatedAt":1,"sunshinePort":47989}"#
            .to_vec()
    }

    fn probe(status: u16, body: Vec<u8>) -> Probe {
        Probe { addr: "100.1.2.3".into(), status, etag: Some("e1".into()), body, rtt_ms: 40 }
    }

    fn online() -> Live {
        next_live(&Live::default(), Some(&probe(200, presence_body())))
    }

    #[test]
    fn answer_makes_friend_online_with_address_and_latency() {
        let l = online();
        assert!(l.presence.is_some());
        assert_eq!(l.address.as_deref(), Some("100.1.2.3"));
        assert_eq!((l.rtt_ms, l.error, l.misses), (Some(40), None, 0));
        // 304: stessa presenza, latenza aggiornata
        let mut p = probe(304, Vec::new());
        p.rtt_ms = 55;
        let again = next_live(&l, Some(&p));
        assert_eq!((again.presence, again.rtt_ms), (l.presence, Some(55)));
    }

    #[test]
    fn one_lost_answer_does_not_make_friend_offline() {
        let once = next_live(&online(), None);
        assert!(once.presence.is_some(), "un giro a vuoto non basta");
        assert_eq!(once.misses, 1);
        let twice = next_live(&once, None);
        assert!(twice.presence.is_none() && twice.address.is_none());
        assert_eq!(twice.error, Some(0));
    }

    #[test]
    fn http_errors_and_bad_bodies_are_offline_at_once() {
        let denied = next_live(&online(), Some(&probe(403, b"{}".to_vec())));
        assert!(denied.presence.is_none());
        assert_eq!(denied.error, Some(403));
        let garbage = next_live(&Live::default(), Some(&probe(200, b"<html>".to_vec())));
        assert!(garbage.presence.is_none() && garbage.etag.is_none());
        // 304 senza presenza in memoria: niente stato inventato
        let stale = next_live(&Live::default(), Some(&probe(304, Vec::new())));
        assert!(stale.presence.is_none() && stale.address.is_none());
    }

    #[test]
    fn announce_only_on_403_and_not_too_often() {
        let p403 = probe(403, Vec::new());
        assert!(should_announce(&Live::default(), Some(&p403), 1000));
        assert!(!should_announce(&Live::default(), Some(&probe(401, Vec::new())), 1000));
        assert!(!should_announce(&Live::default(), None, 1000));
        let recent = Live { device_sent_at: Some(1000), ..Live::default() };
        assert!(!should_announce(&recent, Some(&p403), 1000 + DEVICE_RETRY_S - 1));
        assert!(should_announce(&recent, Some(&p403), 1000 + DEVICE_RETRY_S));
    }
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
