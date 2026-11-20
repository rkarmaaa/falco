//! Server JSON-RPC 2.0 per la UI. Una riga per messaggio; eventi in broadcast.

use falco_proto::ipc::{Incoming, Request, Response, RpcError, codes, events, methods, to_line};
use falco_proto::transport::{self, RecvHalf, SendHalf};
use interprocess::local_socket::traits::tokio::Listener as _;
use serde::Deserialize;
use serde_json::{Value, json};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

use crate::core::Shared;

pub async fn serve(core: Shared) {
    let listener = match transport::listen() {
        Ok(l) => l,
        Err(e) => {
            tracing::error!("IPC non disponibile: {e}");
            return;
        }
    };
    loop {
        match listener.accept().await {
            Ok(stream) => {
                let core = core.clone();
                tokio::spawn(async move {
                    let (rx, tx) = transport::split(stream);
                    core.lock().await.ui_clients += 1;
                    core.lock().await.poke.notify_one();
                    // Finestra aperta: inviti e richieste subito, non al prossimo giro del Hub
                    crate::hub::poke();
                    if let Err(e) = connection(core.clone(), rx, tx).await {
                        tracing::debug!("connessione IPC chiusa: {e}");
                    }
                    let mut c = core.lock().await;
                    c.ui_clients = c.ui_clients.saturating_sub(1);
                });
            }
            Err(e) => tracing::warn!("accept IPC: {e}"),
        }
    }
}

async fn connection(core: Shared, rx: RecvHalf, mut tx: SendHalf) -> std::io::Result<()> {
    let mut lines = BufReader::new(rx).lines();
    let mut events = core.lock().await.subscribe();
    loop {
        tokio::select! {
            line = lines.next_line() => {
                let Some(line) = line? else { return Ok(()) };
                if let Some(resp) = handle_line(&core, &line).await {
                    tx.write_all(&to_line(&resp)?).await?;
                }
            }
            ev = events.recv() => {
                if let Ok(n) = ev {
                    tx.write_all(&to_line(&n)?).await?;
                }
            }
        }
    }
}

async fn handle_line(core: &Shared, line: &str) -> Option<Response> {
    let req: Request = match serde_json::from_str(line) {
        Ok(r) => r,
        Err(_) if serde_json::from_str::<Incoming>(line).is_ok() => return None,
        Err(e) => {
            return Some(Response::err(
                Value::Null,
                RpcError::new(codes::PARSE_ERROR, e.to_string()),
            ));
        }
    };
    let id = req.id.clone();
    let method = req.method.clone();
    let result = dispatch(core, req).await;
    if let Err(e) = &result {
        core.lock().await.errors.push(&method, Some(e.code), &e.message, e.data.clone());
    }
    // Le notifiche (senza id) non ricevono risposta
    let id = id?;
    Some(match result {
        Ok(v) => Response::ok(id, v),
        Err(e) => Response::err(id, e),
    })
}

fn params<T: for<'de> Deserialize<'de>>(v: Value) -> Result<T, RpcError> {
    serde_json::from_value(v).map_err(|e| RpcError::new(codes::INVALID_PARAMS, e.to_string()))
}

fn internal(e: impl std::fmt::Display) -> RpcError {
    RpcError::new(codes::INTERNAL, e.to_string())
}

fn config_error(e: falco_store::ConfigError) -> RpcError {
    use falco_store::ConfigError as E;
    let data = match &e {
        E::Invalid { key, value, allowed, line } => {
            json!({ "key": key, "value": value, "allowed": allowed, "line": line })
        }
        E::UnknownKey(k) => json!({ "key": k }),
        _ => Value::Null,
    };
    RpcError::new(codes::CONFIG_INVALID, e.to_string()).with_data(data)
}

#[derive(Deserialize)]
struct SetConfig {
    key: String,
    value: Value,
}

#[derive(Deserialize)]
struct ImportConfig {
    text: String,
}

#[derive(Deserialize)]
struct ResetConfig {
    section: String,
}

#[derive(Deserialize)]
struct SetAvailable {
    available: bool,
}

#[derive(Deserialize)]
struct Approve {
    id: String,
    pin: String,
    name: String,
}

#[derive(Deserialize)]
struct ClientRef {
    uuid: String,
    #[serde(default)]
    enabled: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct HostTarget {
    #[serde(default)]
    host: String,
    #[serde(default)]
    port: Option<u16>,
    /// In alternativa a host/porta: l'amico da raggiungere.
    #[serde(default)]
    friend_id: Option<String>,
    #[serde(default = "desktop")]
    app: String,
}

#[derive(Deserialize)]
struct FriendRef {
    id: String,
    #[serde(default)]
    favorite: bool,
}

#[derive(Deserialize)]
struct Link {
    link: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct InviteDecision {
    request_id: String,
    accept: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct TokenRef {
    token_hash: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct OAuth {
    client_id: String,
    client_secret: String,
}

#[derive(Deserialize)]
struct HubText {
    text: String,
}

#[derive(Deserialize)]
struct OpenWeb {
    /// `site` | `github`
    target: String,
}

#[derive(Deserialize)]
struct HubId {
    id: String,
}

#[derive(Deserialize)]
struct CreateProfile {
    name: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ProfileEdit {
    name: Option<String>,
    status: Option<falco_proto::Status>,
    /// Immagine in base64 (qualsiasi formato comune).
    avatar_base64: Option<String>,
    #[serde(default)]
    clear_avatar: bool,
}

/// Amico non raggiungibile, con il motivo per la UI: `offline` (nessuna risposta) o
/// `unrecognized` (risponde ma non riconosce questo PC, per esempio dopo un nodo Tailscale nuovo).
async fn unreachable(core: &Shared, id: &str) -> RpcError {
    let error = core.lock().await.live.get(id).and_then(|l| l.error);
    let reason = if error == Some(403) { "unrecognized" } else { "offline" };
    RpcError::new(codes::SESSION_FAILED, "amico non raggiungibile")
        .with_data(json!({ "reason": reason }))
}

/// Indirizzo di presenza di un amico, chiesto subito se non è noto.
async fn friend_address(core: &Shared, id: &str) -> Result<String, RpcError> {
    match crate::social::reach(core, id).await {
        Some(a) => Ok(a),
        None => Err(unreachable(core, id).await),
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SetPermissions {
    id: String,
    role: Option<falco_proto::Role>,
    overrides: Option<falco_proto::permissions::PermissionOverrides>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct KnockDecision {
    friend_id: String,
    accept: bool,
}

/// «Entra» da un amico: bussa, poi avvia lo stream (usato da UI e tray). Moonlight si collega allo
/// stesso IP che ha risposto alla presenza, sulla porta di Sunshine che l'amico ha annunciato.
pub async fn join_friend(core: &Shared, id: &str, app: &str) -> Result<(), RpcError> {
    let addr = friend_address(core, id).await?;
    let permissions = knock_friend(core, &addr).await?;
    let (port, name) = {
        let mut c = core.lock().await;
        c.joined = Some(id.to_owned());
        let port = c.live.get(id).and_then(|l| l.presence.as_ref()).map(|p| p.sunshine_port);
        (
            port.unwrap_or(falco_proto::fpp::DEFAULT_SUNSHINE_PORT),
            c.db.friend(id).ok().flatten().map(|f| f.name),
        )
    };
    let (host, _) = crate::social::split_address(&addr);
    let address = crate::guest::moonlight_address(&host, port);
    let info = crate::guest::SessionInfo { friend_id: Some(id.to_owned()), name, permissions };
    let started = crate::guest::join(core.clone(), address, app.to_owned(), info).await;
    if started.is_err() {
        // Moonlight non è partito: nessuna sessione da chiudere più tardi
        core.lock().await.joined = None;
    }
    started.map_err(anyhow_err(codes::SESSION_FAILED))
}

/// Bussa all'host prima di avviare Moonlight (§7); errore con il motivo se rifiutato.
async fn knock_friend(core: &Shared, addr: &str) -> Result<Option<Value>, RpcError> {
    let (me, wait) = {
        let c = core.lock().await;
        let me = c.me.as_ref().map(|m| m.identity.clone());
        // Tempo dell'host per rispondere, per avviare la condivisione e margine di rete
        let host_wait = u64::from(c.config.config().host.knock_timeout_s);
        (me, host_wait + crate::access::HOST_STARTUP_WAIT_S + 15)
    };
    let me = me.ok_or_else(|| RpcError::new(codes::SESSION_FAILED, "profilo non pronto"))?;
    let addr = addr.to_owned();
    let (status, _, body) = tokio::task::spawn_blocking(move || {
        crate::social::fpp_call(
            &me,
            "POST",
            &addr,
            "/v1/session/knock",
            Some(&json!({})),
            None,
            std::time::Duration::from_secs(wait),
        )
    })
    .await
    .map_err(internal)?
    .map_err(anyhow_err(codes::SESSION_FAILED))?;
    if status != 200 {
        return Err(RpcError::new(codes::SESSION_FAILED, format!("l'host ha risposto {status}")));
    }
    match serde_json::from_slice::<crate::access::KnockAnswer>(&body) {
        Ok(crate::access::KnockAnswer::Accepted { permissions }) => {
            Ok(serde_json::to_value(permissions).ok())
        }
        Ok(crate::access::KnockAnswer::Declined { reason }) => {
            Err(RpcError::new(codes::SESSION_FAILED, format!("ingresso non consentito ({reason})"))
                .with_data(json!({ "reason": reason })))
        }
        Err(e) => Err(internal(e)),
    }
}

fn anyhow_err(code: i32) -> impl Fn(anyhow::Error) -> RpcError {
    move |e| RpcError::new(code, format!("{e:#}"))
}

fn desktop() -> String {
    "Desktop".into()
}

/// Chiamata bloccante all'API di Sunshine, fuori dal runtime.
async fn sunshine<T: Send + 'static>(
    core: &Shared,
    f: impl FnOnce(
        &falco_engines::sunshine_api::SunshineApi,
    ) -> Result<T, falco_engines::sunshine_api::ApiError>
    + Send
    + 'static,
) -> Result<T, RpcError> {
    let api = core
        .lock()
        .await
        .host
        .api()
        .ok_or_else(|| RpcError::new(codes::HOST_NOT_READY, "l'host non è attivo"))?;
    tokio::task::spawn_blocking(move || f(&api)).await.map_err(internal)?.map_err(internal)
}

async fn dispatch(core: &Shared, req: Request) -> Result<Value, RpcError> {
    // Metodi che gestiscono da soli il lock o lavorano in background
    match req.method.as_str() {
        methods::HOST_SET_AVAILABLE => {
            let p: SetAvailable = params(req.params)?;
            core.lock().await.user_available = p.available;
            tokio::spawn(crate::core::set_available(core.clone(), p.available));
            return Ok(json!({ "available": p.available }));
        }
        methods::HOST_PAIRINGS => {
            return serde_json::to_value(sunshine(core, |a| a.pending_pairings()).await?)
                .map_err(internal);
        }
        methods::HOST_APPROVE => {
            let p: Approve = params(req.params)?;
            sunshine(core, move |a| a.submit_pin(&p.id, &p.pin, &p.name)).await?;
            return Ok(Value::Null);
        }
        methods::HOST_CLIENTS => {
            return serde_json::to_value(sunshine(core, |a| a.clients()).await?).map_err(internal);
        }
        methods::HOST_UNPAIR => {
            let p: ClientRef = params(req.params)?;
            sunshine(core, move |a| a.unpair(&p.uuid)).await?;
            return Ok(Value::Null);
        }
        methods::HOST_SET_CLIENT_ENABLED => {
            let p: ClientRef = params(req.params)?;
            sunshine(core, move |a| a.set_enabled(&p.uuid, p.enabled)).await?;
            return Ok(Value::Null);
        }
        methods::HOST_END_SESSION => {
            sunshine(core, |a| a.close_app()).await?;
            return Ok(Value::Null);
        }
        methods::SESSION_PAIR => {
            let p: HostTarget = params(req.params)?;
            let address = crate::guest::moonlight_address(&p.host, p.port.unwrap_or(47989));
            let pin = crate::guest::new_pin();
            let (core2, pin2, addr2) = (core.clone(), pin.clone(), address.clone());
            tokio::spawn(async move {
                if let Err(e) = crate::guest::pair(core2.clone(), addr2.clone(), pin2).await {
                    core2.lock().await.record_error(
                        "session.pair",
                        &format!("{e:#}"),
                        Some(json!({ "host": addr2 })),
                    );
                }
            });
            return Ok(json!({ "pin": pin, "host": address }));
        }
        methods::SESSION_JOIN => {
            let p: HostTarget = params(req.params)?;
            if let Some(id) = &p.friend_id {
                join_friend(core, id, &p.app).await?;
                return Ok(Value::Null);
            }
            let (host, port) = (p.host.clone(), p.port.unwrap_or(47989));
            let address = crate::guest::moonlight_address(&host, port);
            crate::guest::join(core.clone(), address, p.app, crate::guest::SessionInfo::default())
                .await
                .map_err(anyhow_err(codes::SESSION_FAILED))?;
            return Ok(Value::Null);
        }
        methods::INVITE_CREATE => {
            let p: crate::invites::CreateParams = params(req.params)?;
            let created =
                crate::invites::create(core, p).await.map_err(anyhow_err(codes::INTERNAL))?;
            return serde_json::to_value(created).map_err(internal);
        }
        methods::INVITE_REVOKE => {
            let p: TokenRef = params(req.params)?;
            crate::invites::revoke(core, &p.token_hash)
                .await
                .map_err(anyhow_err(codes::INTERNAL))?;
            return Ok(Value::Null);
        }
        methods::INVITE_ACCEPT => {
            let p: Link = params(req.params)?;
            // Valida subito il link, poi prosegue in background con eventi invite.progress
            crate::invites::preview(&p.link).map_err(anyhow_err(codes::INVALID_PARAMS))?;
            if !crate::invites::begin_accept(&p.link) {
                // Già in corso (link aperto due volte): i progressi arrivano dal primo tentativo
                return Ok(Value::Null);
            }
            let core2 = core.clone();
            tokio::spawn(async move {
                if let Err(e) = crate::invites::accept(core2.clone(), p.link).await {
                    core2.lock().await.errors.push("invite.accept", None, &format!("{e:#}"), None);
                    core2.lock().await.emit(
                        events::INVITE_PROGRESS,
                        json!({ "side": "guest", "step": "error", "ok": false, "message": format!("{e:#}") }),
                    );
                }
            });
            return Ok(Value::Null);
        }
        methods::TAILSCALE_SET_OAUTH => {
            let p: OAuth = params(req.params)?;
            tokio::task::spawn_blocking(move || {
                crate::invites::set_oauth(&p.client_id, &p.client_secret)
            })
            .await
            .map_err(internal)?
            .map_err(anyhow_err(codes::INVALID_PARAMS))?;
            return Ok(Value::Null);
        }
        methods::FRIEND_SET_PERMISSIONS => {
            let p: SetPermissions = params(req.params)?;
            {
                let c = core.lock().await;
                let current = c.db.permissions(&p.id).map_err(internal)?;
                let role = p
                    .role
                    .map(crate::invites::role_name)
                    .or(current.as_ref().map(|x| x.0.clone()))
                    .unwrap_or_else(|| "spettatore".into());
                let overrides = match &p.overrides {
                    Some(o) => serde_json::to_string(o).map_err(internal)?,
                    None => current.map(|x| x.1).unwrap_or_else(|| "{}".into()),
                };
                c.db.set_permissions(&p.id, &role, &overrides, crate::core::now())
                    .map_err(internal)?;
                crate::social::emit_friends(&c);
            }
            // Effetto lato host senza riconnessione; gli input richiedono il riavvio del motore
            crate::access::apply_all(core).await;
            crate::access::sync_inputs(core).await;
            crate::access::push_permissions(core, &p.id).await;
            return Ok(Value::Null);
        }
        methods::HOST_ANSWER_KNOCK => {
            let p: KnockDecision = params(req.params)?;
            if let Some(tx) = core.lock().await.knocks.remove(&p.friend_id) {
                let _ = tx.send(p.accept);
            }
            return Ok(Value::Null);
        }
        methods::SESSION_LEAVE => {
            if let Some(stop) = core.lock().await.session_stop.take() {
                let _ = stop.send(());
            }
            return Ok(Value::Null);
        }
        methods::HOST_KICK => {
            let p: FriendRef = params(req.params)?;
            crate::access::kick(core, &p.id).await;
            return Ok(Value::Null);
        }
        methods::FRIEND_NOTIFY => {
            let p: FriendRef = params(req.params)?;
            let addr = friend_address(core, &p.id).await?;
            let me = core.lock().await.me.as_ref().map(|m| m.identity.clone());
            let me =
                me.ok_or_else(|| RpcError::new(codes::SESSION_FAILED, "profilo non pronto"))?;
            tokio::task::spawn_blocking(move || {
                crate::social::fpp_call(
                    &me,
                    "POST",
                    &addr,
                    "/v1/session/notify",
                    Some(&json!({})),
                    None,
                    std::time::Duration::from_secs(5),
                )
            })
            .await
            .map_err(internal)?
            .map_err(anyhow_err(codes::SESSION_FAILED))?;
            return Ok(Value::Null);
        }
        methods::NET_RUN_DOCTOR => {
            return Ok(crate::doctor::to_json(&crate::doctor::run(core).await));
        }
        methods::HOST_RECOMMEND => {
            let checks = crate::doctor::run(core).await;
            let (profile, reasons) = crate::doctor::recommend(&checks);
            return Ok(json!({ "profile": profile, "reasons": reasons, "checks": checks }));
        }
        methods::HOST_APPLY_INPUTS => {
            // Riavvia subito la condivisione anche con ospiti collegati (si ricollegano)
            {
                let mut c = core.lock().await;
                c.guests.clear();
                crate::access::emit_guests(&mut c);
            }
            crate::access::sync_inputs(core).await;
            return Ok(Value::Null);
        }
        methods::DEBUG_REPORT => {
            let logs = core.lock().await.data_dir.join("logs");
            return Ok(crate::debug::report(core, &logs).await);
        }
        methods::UPDATE_CHECK => {
            let a = crate::updater::check(core).await.map_err(anyhow_err(codes::INTERNAL))?;
            return Ok(json!({
                "available": a,
                "current": env!("CARGO_PKG_VERSION"),
                "enabled": !crate::updater::PUBLIC_KEY.is_empty(),
            }));
        }
        methods::UPDATE_INSTALL => {
            crate::updater::install(core).await.map_err(anyhow_err(codes::INTERNAL))?;
            return Ok(Value::Null);
        }
        methods::TAILSCALE_INSTALL => {
            let dir = core.lock().await.data_dir.join("downloads");
            std::fs::create_dir_all(&dir).map_err(internal)?;
            crate::platform::install_tailscale(&dir).await.map_err(anyhow_err(codes::INTERNAL))?;
            let mut c = core.lock().await;
            c.net = falco_net::tailscale::Tailscale::detect();
            c.emit(events::PRESENCE_CHANGED, json!({ "self": { "tailscale": true } }));
            return Ok(Value::Null);
        }
        methods::HUB_INFO => return Ok(crate::hub::info(core).await),
        methods::HUB_REQUESTS => {
            let r = crate::hub::requests(core).await.map_err(anyhow_err(codes::INTERNAL))?;
            return serde_json::to_value(r).map_err(internal);
        }
        methods::HUB_ADD_FRIEND => {
            let p: HubText = params(req.params)?;
            return crate::hub::add_friend(core, &p.text)
                .await
                .map_err(anyhow_err(codes::INTERNAL));
        }
        methods::HUB_ACCEPT => {
            let p: HubId = params(req.params)?;
            let a = crate::hub::accept(core, &p.id).await.map_err(anyhow_err(codes::INTERNAL))?;
            return serde_json::to_value(a).map_err(internal);
        }
        methods::HUB_DECLINE => {
            let p: HubId = params(req.params)?;
            crate::hub::decline(core, &p.id).await.map_err(anyhow_err(codes::INTERNAL))?;
            return Ok(Value::Null);
        }
        methods::HUB_SHARE_STREAM => {
            let p: crate::invites::CreateParams = params(req.params)?;
            let link =
                crate::hub::share_stream(core, p).await.map_err(anyhow_err(codes::INTERNAL))?;
            return serde_json::to_value(link).map_err(internal);
        }
        methods::HUB_JOIN => {
            let p: HubText = params(req.params)?;
            let a = crate::hub::join(core, &p.text).await.map_err(anyhow_err(codes::INTERNAL))?;
            return serde_json::to_value(a).map_err(internal);
        }
        methods::HUB_FRIENDS => {
            let f = crate::hub::friends(core).await.map_err(anyhow_err(codes::INTERNAL))?;
            return serde_json::to_value(f).map_err(internal);
        }
        methods::HUB_INVITE_FRIEND => {
            let p: HubId = params(req.params)?;
            crate::hub::invite_friend(core, &p.id).await.map_err(anyhow_err(codes::INTERNAL))?;
            return Ok(Value::Null);
        }
        methods::HUB_TAKE_INVITE => {
            let p: HubId = params(req.params)?;
            let a =
                crate::hub::take_invite(core, &p.id).await.map_err(anyhow_err(codes::INTERNAL))?;
            return serde_json::to_value(a).map_err(internal);
        }
        methods::HUB_DECLINE_INVITE => {
            let p: HubId = params(req.params)?;
            crate::hub::decline_invite(core, &p.id).await.map_err(anyhow_err(codes::INTERNAL))?;
            return Ok(Value::Null);
        }
        methods::APP_OPEN_WEB => {
            let p: OpenWeb = params(req.params)?;
            let url = match p.target.as_str() {
                "site" => crate::hub::base_url(core.lock().await.config.config())
                    .unwrap_or_else(|| crate::hub::SITE_URL.into()),
                "github" => crate::hub::REPO_URL.into(),
                _ => return Err(RpcError::new(codes::INVALID_PARAMS, "link sconosciuto")),
            };
            opener::open_browser(&url).map_err(internal)?;
            return Ok(Value::Null);
        }
        methods::NET_JOIN_FALCO => {
            crate::hub::join_network(core).await.map_err(anyhow_err(codes::INTERNAL))?;
            return Ok(Value::Null);
        }
        methods::NET_LOGIN_HEADSCALE => {
            // Stesso client Tailscale, server di controllo personalizzato (§9)
            let (ts, url) = {
                let c = core.lock().await;
                (c.net.clone(), c.config.config().network.headscale_url.clone())
            };
            if !url.starts_with("https://") {
                return Err(RpcError::new(
                    codes::INVALID_PARAMS,
                    "imposta prima l'URL di Headscale",
                ));
            }
            let out = ts.run(&["login", "--login-server", &url]).await.map_err(internal)?;
            return Ok(json!({ "output": String::from_utf8_lossy(&out) }));
        }
        methods::FRIEND_REMOVE => {
            let p: FriendRef = params(req.params)?;
            crate::social::remove_friend(core, &p.id, false).await;
            return Ok(Value::Null);
        }
        methods::FRIEND_BLOCK => {
            let p: FriendRef = params(req.params)?;
            crate::social::remove_friend(core, &p.id, true).await;
            return Ok(Value::Null);
        }
        _ => {}
    }

    let mut c = core.lock().await;
    // Il portachiavi può non essere pronto all'avvio del core: si riprova a ogni richiesta
    if c.me.is_none()
        && matches!(req.method.as_str(), methods::PROFILE_GET | methods::PROFILE_CREATE)
    {
        match crate::identity::Me::load(&c.db) {
            Ok(me) => c.me = me,
            Err(e) => return Err(anyhow_err(codes::INTERNAL)(e)),
        }
    }
    match req.method.as_str() {
        methods::PROFILE_GET => Ok(match c.me.as_ref() {
            None => json!({ "exists": false }),
            Some(m) => json!({
                "exists": true,
                "id": m.identity.id().to_string(),
                "tag": m.identity.id().tag(),
                "accent": m.identity.id().accent(),
                "name": m.name(),
                "avatarUrl": m.avatar_hash()
                    .and_then(|h| c.db.avatar(h).ok().flatten())
                    .map(|b| crate::identity::data_url(&b)),
            }),
        }),
        methods::PROFILE_CREATE => {
            let p: CreateProfile = params(req.params)?;
            // Profilo già presente (UI rimasta indietro): niente errore, la UI lo ricarica
            if c.me.is_some() {
                return Ok(Value::Null);
            }
            let me = crate::identity::Me::create(&c.db, &p.name, crate::core::now())
                .map_err(anyhow_err(codes::INVALID_PARAMS))?;
            c.me = Some(me);
            crate::hub::forget_registration();
            crate::hub::poke();
            c.emit(events::PRESENCE_CHANGED, json!({ "self": { "hasProfile": true } }));
            Ok(Value::Null)
        }
        methods::FRIENDS_LIST => {
            serde_json::to_value(crate::social::friend_views(&c)).map_err(internal)
        }
        methods::FRIENDS_REFRESH => {
            c.poke.notify_one();
            Ok(Value::Null)
        }
        methods::FRIEND_FAVORITE => {
            let p: FriendRef = params(req.params)?;
            c.db.set_favorite(&p.id, p.favorite).map_err(internal)?;
            crate::social::emit_friends(&c);
            Ok(Value::Null)
        }
        methods::INVITE_LIST => {
            serde_json::to_value(c.db.invites(crate::core::now()).map_err(internal)?)
                .map_err(internal)
        }
        methods::INVITE_PREVIEW => {
            let p: Link = params(req.params)?;
            crate::invites::preview(&p.link).map_err(anyhow_err(codes::INVALID_PARAMS))
        }
        methods::INVITE_APPROVE => {
            let p: InviteDecision = params(req.params)?;
            if let Some(tx) = c.approvals.remove(&p.request_id) {
                let _ = tx.send(p.accept);
            }
            Ok(Value::Null)
        }
        methods::STATE_SNAPSHOT => {
            let net = crate::core::Core::net_status(c.net.clone()).await;
            serde_json::to_value(c.snapshot(&net)).map_err(internal)
        }
        methods::CONFIG_SCHEMA => {
            serde_json::to_value(falco_store::schema::fields()).map_err(internal)
        }
        methods::CONFIG_EXPORT => Ok(json!({ "text": c.config.text() })),
        methods::CONFIG_IMPORT => {
            let p: ImportConfig = params(req.params)?;
            c.config.replace_text(&p.text).map_err(config_error)?;
            c.emit(events::CONFIG_CHANGED, json!({ "section": "*" }));
            c.refresh_tray();
            Ok(Value::Null)
        }
        methods::NET_LAN_PEERS => {
            serde_json::to_value(c.lan_peers.values().collect::<Vec<_>>()).map_err(internal)
        }
        methods::CONFIG_GET => {
            let cfg = c.config.config();
            Ok(json!({ "config": cfg }))
        }
        methods::CONFIG_SET => {
            let p: SetConfig = params(req.params)?;
            let at = c.config.set(&p.key, &p.value).map_err(config_error)?;
            c.emit(events::CONFIG_CHANGED, json!({ "key": p.key, "value": p.value }));
            c.refresh_tray();
            Ok(json!({ "applyAt": at }))
        }
        methods::CONFIG_RESET => {
            let p: ResetConfig = params(req.params)?;
            c.config.reset_section(&p.section).map_err(config_error)?;
            c.emit(events::CONFIG_CHANGED, json!({ "section": p.section }));
            c.refresh_tray();
            Ok(Value::Null)
        }
        methods::PROFILE_UPDATE => {
            let p: ProfileEdit = params(req.params)?;
            if let Some(s) = p.status {
                c.set_status(s).map_err(config_error)?;
            }
            let now = crate::core::now();
            let core = &mut *c;
            if let Some(me) = core.me.as_mut() {
                if let Some(name) = &p.name {
                    me.rename(&core.db, name, now).map_err(anyhow_err(codes::INVALID_PARAMS))?;
                    crate::hub::forget_registration();
                }
                if let Some(b64) = &p.avatar_base64 {
                    let bytes = data_encoding::BASE64
                        .decode(b64.as_bytes())
                        .map_err(|_| RpcError::new(codes::INVALID_PARAMS, "immagine non valida"))?;
                    me.set_avatar(&core.db, &bytes, now)
                        .map_err(anyhow_err(codes::INVALID_PARAMS))?;
                }
                if p.clear_avatar {
                    me.clear_avatar(&core.db, now).map_err(anyhow_err(codes::INTERNAL))?;
                }
            }
            if p.name.is_some() || p.avatar_base64.is_some() || p.clear_avatar {
                c.emit(events::PRESENCE_CHANGED, json!({ "self": { "profile": true } }));
            }
            Ok(Value::Null)
        }
        methods::APP_OPEN_LINK => {
            let p: Link = params(req.params)?;
            c.open_link(&p.link);
            Ok(Value::Null)
        }
        methods::APP_OPEN_UI => {
            c.open_ui();
            Ok(Value::Null)
        }
        methods::FRIEND_GET_PERMISSIONS => {
            let p: FriendRef = params(req.params)?;
            let raw = c.db.permissions(&p.id).map_err(internal)?;
            Ok(json!({
                "role": raw.as_ref().map(|r| r.0.clone()),
                "overrides": raw.as_ref().and_then(|r| serde_json::from_str::<Value>(&r.1).ok()),
                "effective": crate::access::effective(&c, &p.id),
                "paired": raw.and_then(|r| r.2).is_some(),
            }))
        }
        methods::HOST_GUESTS => Ok(json!({ "guests": c.guests.values().collect::<Vec<_>>() })),
        other => {
            Err(RpcError::new(codes::METHOD_NOT_FOUND, format!("metodo sconosciuto: {other}")))
        }
    }
}
