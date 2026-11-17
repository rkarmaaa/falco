//! Server JSON-RPC 2.0 per la UI. Una riga per messaggio; eventi in broadcast.

use falo_proto::ipc::{Incoming, Request, Response, RpcError, codes, events, methods, to_line};
use falo_proto::transport::{self, RecvHalf, SendHalf};
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
                    if let Err(e) = connection(core, rx, tx).await {
                        tracing::debug!("connessione IPC chiusa: {e}");
                    }
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
    let result = dispatch(core, req).await;
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

fn not_yet(milestone: &str) -> RpcError {
    RpcError::new(codes::NOT_IMPLEMENTED, "non ancora disponibile")
        .with_data(json!({ "milestone": milestone }))
}

fn internal(e: impl std::fmt::Display) -> RpcError {
    RpcError::new(codes::INTERNAL, e.to_string())
}

fn config_error(e: falo_store::ConfigError) -> RpcError {
    use falo_store::ConfigError as E;
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
struct ResetConfig {
    section: String,
}

#[derive(Deserialize)]
struct SetAvailable {
    available: bool,
}

#[derive(Deserialize)]
struct ProfileUpdate {
    status: Option<falo_proto::Status>,
}

async fn dispatch(core: &Shared, req: Request) -> Result<Value, RpcError> {
    let mut c = core.lock().await;
    match req.method.as_str() {
        methods::STATE_SNAPSHOT => {
            let net = crate::core::Core::net_status(c.net.clone()).await;
            serde_json::to_value(c.snapshot(&net)).map_err(internal)
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
            let p: ProfileUpdate = params(req.params)?;
            if let Some(s) = p.status {
                c.set_status(s).map_err(config_error)?;
            }
            Ok(Value::Null)
        }
        methods::HOST_SET_AVAILABLE => {
            let p: SetAvailable = params(req.params)?;
            c.set_available(p.available);
            Ok(json!({ "available": p.available }))
        }
        methods::FRIENDS_LIST => {
            serde_json::to_value(c.db.friends().map_err(internal)?).map_err(internal)
        }
        methods::NET_RUN_DOCTOR => {
            let status = crate::core::Core::net_status(c.net.clone()).await;
            let warn = c.config.config().network.warn_exit_node;
            Ok(json!({ "checks": falo_net::doctor::tailscale_checks(&status, warn) }))
        }
        methods::APP_OPEN_UI => {
            c.open_ui();
            Ok(Value::Null)
        }
        methods::FRIEND_SET_PERMISSIONS => Err(not_yet("M4")),
        methods::INVITE_CREATE | methods::INVITE_ACCEPT => Err(not_yet("M3")),
        methods::SESSION_JOIN => Err(not_yet("M1")),
        other => {
            Err(RpcError::new(codes::METHOD_NOT_FOUND, format!("metodo sconosciuto: {other}")))
        }
    }
}
