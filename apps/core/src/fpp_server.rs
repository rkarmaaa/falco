//! Server di presenza FPP v1 (§5): solo sugli indirizzi Tailscale, ogni richiesta firmata.

use std::collections::HashMap;
use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
use std::time::{Duration, Instant};

use axum::Router;
use axum::body::Bytes;
use axum::extract::{ConnectInfo, Path, State};
use axum::http::{HeaderMap, Method, StatusCode, Uri, header};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use falco_proto::FalcoId;
use falco_proto::fpp::{self, HandshakeRequest, HandshakeResponse};
use falco_proto::ipc::events;
use falco_store::db::{DeviceRow, FriendRow, FriendState};
use serde_json::json;
use sha2::Digest;
use tokio::sync::Mutex;

use crate::core::{Shared, now};
use crate::social;

const RATE_PER_MIN: u32 = 20;

/// Valori per IP con l'istante di inserimento.
type PerIp<V> = Arc<Mutex<HashMap<IpAddr, (Instant, V)>>>;
/// Richiesta rifiutata: codice e motivo.
type Denied = (StatusCode, String);

#[derive(Clone)]
struct Srv {
    core: Shared,
    limiter: PerIp<u32>,
    whois: PerIp<Option<String>>,
}

fn err(code: StatusCode, msg: &str) -> Response {
    (code, axum::Json(json!({ "error": msg }))).into_response()
}

impl Srv {
    /// Massimo 20 richieste al minuto per IP, poi 429.
    async fn rate_ok(&self, ip: IpAddr) -> bool {
        let mut map = self.limiter.lock().await;
        let entry = map.entry(ip).or_insert((Instant::now(), 0));
        if entry.0.elapsed() > Duration::from_secs(60) {
            *entry = (Instant::now(), 0);
        }
        entry.1 += 1;
        entry.1 <= RATE_PER_MIN
    }

    /// Nodo Tailscale dell'IP sorgente, in cache 5 minuti.
    async fn node_of(&self, ip: IpAddr) -> Option<String> {
        if let Some((t, n)) = self.whois.lock().await.get(&ip)
            && t.elapsed() < Duration::from_secs(300)
        {
            return n.clone();
        }
        let ts = self.core.lock().await.net.clone();
        let node = ts.whois(ip).await.ok().flatten();
        self.whois.lock().await.insert(ip, (Instant::now(), node.clone()));
        node
    }

    /// Firma, finestra temporale, frequenza; per gli amici anche nodo registrato.
    async fn auth(
        &self,
        peer: SocketAddr,
        method: &Method,
        uri: &Uri,
        headers: &HeaderMap,
        body: &[u8],
        friends_only: bool,
    ) -> Result<FalcoId, Denied> {
        if !self.rate_ok(peer.ip()).await {
            return Err((StatusCode::TOO_MANY_REQUESTS, "troppe richieste".into()));
        }
        let h = |k: &str| headers.get(k).and_then(|v| v.to_str().ok());
        let id = fpp::verify_request(
            h(fpp::HEADER_ID),
            h(fpp::HEADER_TS),
            h(fpp::HEADER_SIG),
            method.as_str(),
            uri.path(),
            body,
            now(),
        )
        .map_err(|e| (StatusCode::UNAUTHORIZED, e.to_string()))?;
        if !friends_only {
            return Ok(id);
        }
        let key = id.to_string();
        let state = self.core.lock().await.db.friend_state(&key).ok().flatten();
        if !matches!(state, Some(FriendState::Friend | FriendState::Pending)) {
            return Err((StatusCode::FORBIDDEN, "non siete amici".into()));
        }
        // Doppio vincolo: l'IP deve appartenere a un nodo registrato per quell'amico
        let node = self.node_of(peer.ip()).await;
        let owner = match &node {
            Some(n) => self.core.lock().await.db.friend_by_node(n).ok().flatten(),
            None => None,
        };
        if owner.as_deref() != Some(key.as_str()) {
            return Err((StatusCode::FORBIDDEN, "dispositivo non riconosciuto".into()));
        }
        Ok(id)
    }
}

async fn profile(
    State(s): State<Srv>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    method: Method,
    uri: Uri,
    headers: HeaderMap,
) -> Response {
    if let Err((code, msg)) = s.auth(peer, &method, &uri, &headers, b"", true).await {
        return err(code, &msg);
    }
    let ts = s.core.lock().await.net.clone();
    let net = crate::core::Core::net_status(ts).await;
    let c = s.core.lock().await;
    match c.me.as_ref().map(|m| m.signed(c.status(), social::my_devices(&net))) {
        Some(Ok(p)) => axum::Json(p).into_response(),
        _ => err(StatusCode::SERVICE_UNAVAILABLE, "profilo non pronto"),
    }
}

async fn avatar(
    State(s): State<Srv>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    method: Method,
    uri: Uri,
    headers: HeaderMap,
    Path(hash): Path<String>,
) -> Response {
    if let Err((code, msg)) = s.auth(peer, &method, &uri, &headers, b"", true).await {
        return err(code, &msg);
    }
    match s.core.lock().await.db.avatar(&hash).ok().flatten() {
        Some(bytes) => (
            [
                (header::CONTENT_TYPE, "image/webp"),
                (header::CACHE_CONTROL, "max-age=31536000, immutable"),
            ],
            bytes,
        )
            .into_response(),
        None => err(StatusCode::NOT_FOUND, "avatar sconosciuto"),
    }
}

async fn presence(
    State(s): State<Srv>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    method: Method,
    uri: Uri,
    headers: HeaderMap,
) -> Response {
    if let Err((code, msg)) = s.auth(peer, &method, &uri, &headers, b"", true).await {
        return err(code, &msg);
    }
    let p = social::my_presence(&*s.core.lock().await);
    let body = serde_json::to_vec(&p).unwrap_or_default();
    let etag =
        format!("\"{}\"", &data_encoding::HEXLOWER.encode(&sha2::Sha256::digest(&body))[..16]);
    if headers.get(header::IF_NONE_MATCH).and_then(|v| v.to_str().ok()) == Some(etag.as_str()) {
        return (StatusCode::NOT_MODIFIED, [(header::ETAG, etag)]).into_response();
    }
    ([(header::ETAG, etag.as_str()), (header::CONTENT_TYPE, "application/json")], body)
        .into_response()
}

/// L'amico ci ha rimosso: rifiuto immediato e pulizia locale.
async fn friend_remove(
    State(s): State<Srv>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    method: Method,
    uri: Uri,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    let id = match s.auth(peer, &method, &uri, &headers, &body, true).await {
        Ok(id) => id,
        Err((code, msg)) => return err(code, &msg),
    };
    let key = id.to_string();
    let mut c = s.core.lock().await;
    // Anche il pairing del suo Moonlight smette di valere
    let client = c.db.permissions(&key).ok().flatten().and_then(|p| p.2);
    let api = c.host.api();
    let _ = c.db.remove_friend(&key);
    c.live_remove(&key);
    social::emit_friends(&c);
    drop(c);
    if let (Some(uuid), Some(api)) = (client, api) {
        let _ = tokio::task::spawn_blocking(move || api.unpair(&uuid)).await;
    }
    StatusCode::NO_CONTENT.into_response()
}

/// Conclusione di un invito (§6 passi 5–8). Unico endpoint aperto a non-amici.
async fn handshake(
    State(s): State<Srv>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    method: Method,
    uri: Uri,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    let id = match s.auth(peer, &method, &uri, &headers, &body, false).await {
        Ok(id) => id,
        Err((code, msg)) => return err(code, &msg),
    };
    let Ok(req) = serde_json::from_slice::<HandshakeRequest>(&body) else {
        return err(StatusCode::BAD_REQUEST, "richiesta non valida");
    };
    let Ok(guest) = req.profile.verify().cloned() else {
        return err(StatusCode::BAD_REQUEST, "profilo non valido");
    };
    if guest.id != id {
        return err(StatusCode::UNAUTHORIZED, "profilo di un altro utente");
    }
    // Il nodo dichiarato deve essere quello da cui arriva la richiesta
    if s.node_of(peer.ip()).await.as_deref() != Some(req.node_id.as_str()) {
        return err(StatusCode::FORBIDDEN, "dispositivo non riconosciuto da Tailscale");
    }
    let hash = falco_proto::invite::token_hash(&req.token);
    let consumed = s.core.lock().await.db.consume_invite(&hash, now()).ok().flatten();
    let Some((role, needs_approval)) = consumed else {
        return err(StatusCode::GONE, "invito già usato, revocato o scaduto");
    };

    if needs_approval && !crate::invites::ask_host(&s.core, &guest.name, &guest.tag).await {
        return err(StatusCode::FORBIDDEN, "l'host ha rifiutato");
    }

    let gid = guest.id.to_string();
    let sunshine_port;
    let my_profile;
    {
        let c = s.core.lock().await;
        let row = FriendRow {
            id: gid.clone(),
            tag: guest.tag.clone(),
            name: guest.name.clone(),
            avatar_hash: guest.avatar.clone(),
            accent: guest.accent.clone(),
            state: FriendState::Friend,
            favorite: false,
            added_at: now(),
            last_seen: Some(now()),
        };
        let _ = c.db.upsert_friend(&row);
        let _ =
            c.db.set_friend_profile(&gid, &serde_json::to_string(&req.profile).unwrap_or_default());
        let _ = c.db.set_permissions(&gid, &role, "{}", now());
        let _ = c.db.upsert_device(&DeviceRow {
            friend_id: gid.clone(),
            ts_node_id: req.node_id.clone(),
            ts_ip: social::join_address(&peer.ip().to_string(), req.presence_port),
            hostname: guest.devices.first().map(|d| d.name.clone()).unwrap_or_default(),
            os: guest.devices.first().map(|d| d.os.clone()).unwrap_or_default(),
            can_host: guest.devices.first().is_some_and(|d| d.can_host),
            shared_in: true,
            shared_out: req.reverse_invite_url.is_none(),
        });
        sunshine_port = c.config.config().network.sunshine_base_port;
        my_profile = c.me.as_ref().map(|m| m.signed(c.status(), Vec::new()));
        c.emit(
            events::INVITE_PROGRESS,
            json!({ "side": "host", "step": "friend", "name": guest.name }),
        );
        social::emit_friends(&c);
    }
    let Some(Ok(profile)) = my_profile else {
        return err(StatusCode::SERVICE_UNAVAILABLE, "profilo non pronto");
    };

    // Pairing: Sunshine deve essere attivo; il PIN si conferma in background
    let pin = crate::guest::new_pin();
    if let Err(e) =
        crate::invites::host_pairing(s.core.clone(), gid, guest.name.clone(), pin.clone()).await
    {
        return err(StatusCode::SERVICE_UNAVAILABLE, &format!("host non pronto: {e:#}"));
    }
    axum::Json(HandshakeResponse { profile, pin, sunshine_port }).into_response()
}

/// Un amico chiede di entrare (§7 «Bussare»); long-poll fino al timeout.
async fn knock(
    State(s): State<Srv>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    method: Method,
    uri: Uri,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    let id = match s.auth(peer, &method, &uri, &headers, &body, true).await {
        Ok(id) => id,
        Err((code, msg)) => return err(code, &msg),
    };
    axum::Json(crate::access::knock(&s.core, &id.to_string()).await).into_response()
}

/// L'ospite ha chiuso lo stream.
async fn leave(
    State(s): State<Srv>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    method: Method,
    uri: Uri,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    let id = match s.auth(peer, &method, &uri, &headers, &body, true).await {
        Ok(id) => id,
        Err((code, msg)) => return err(code, &msg),
    };
    crate::access::leave(&s.core, &id.to_string()).await;
    StatusCode::NO_CONTENT.into_response()
}

/// L'host ci invita a giocare adesso.
async fn notify(
    State(s): State<Srv>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    method: Method,
    uri: Uri,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    let id = match s.auth(peer, &method, &uri, &headers, &body, true).await {
        Ok(id) => id,
        Err((code, msg)) => return err(code, &msg),
    };
    let c = s.core.lock().await;
    let name = c.db.friend(&id.to_string()).ok().flatten().map(|f| f.name).unwrap_or_default();
    c.emit(events::PLAY_INVITE, json!({ "friendId": id.to_string(), "name": name }));
    c.notify_user(&format!("{name} ti invita a giocare"), "Apri Falco e premi «Entra».");
    StatusCode::NO_CONTENT.into_response()
}

fn router(s: Srv) -> Router {
    Router::new()
        .route("/v1/profile", get(profile))
        .route("/v1/avatar/{hash}", get(avatar))
        .route("/v1/presence", get(presence))
        .route("/v1/handshake", post(handshake))
        .route("/v1/friend/remove", post(friend_remove))
        .route("/v1/session/knock", post(knock))
        .route("/v1/session/leave", post(leave))
        .route("/v1/session/notify", post(notify))
        .with_state(s)
}

/// Ascolta solo sugli indirizzi Tailscale del dispositivo; si riallinea se cambiano.
pub async fn run(core: Shared) {
    let srv = Srv { core: core.clone(), limiter: Arc::default(), whois: Arc::default() };
    let mut bound: Vec<(IpAddr, u16)> = Vec::new();
    let mut tasks: Vec<tokio::task::JoinHandle<()>> = Vec::new();
    loop {
        let (ts, port) = {
            let c = core.lock().await;
            (c.net.clone(), c.config.config().network.presence_port)
        };
        let net = crate::core::Core::net_status(ts).await;
        let mut want: Vec<(IpAddr, u16)> = net
            .self_ips
            .iter()
            .filter(|ip| social::is_tailnet(**ip))
            .map(|ip| (*ip, port))
            .collect();
        want.sort();
        if want != bound {
            tasks.drain(..).for_each(|t| t.abort());
            for (ip, port) in &want {
                match tokio::net::TcpListener::bind((*ip, *port)).await {
                    Ok(l) => {
                        let app =
                            router(srv.clone()).into_make_service_with_connect_info::<SocketAddr>();
                        tasks.push(tokio::spawn(async move {
                            let _ = axum::serve(l, app).await;
                        }));
                    }
                    Err(e) => tracing::warn!("presenza non in ascolto su {ip}:{port}: {e}"),
                }
            }
            bound = want;
        }
        tokio::time::sleep(Duration::from_secs(60)).await;
    }
}
