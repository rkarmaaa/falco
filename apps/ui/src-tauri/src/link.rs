//! Collegamento persistente al core, con riconnessione automatica.

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::Duration;

use falco_proto::ipc::{Incoming, Request, RpcError, to_line};
use falco_proto::transport::{self, SendHalf};
use serde_json::Value;
use tauri::{AppHandle, Emitter};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::sync::{Mutex, oneshot};

/// Codice riservato alla UI: il core non risponde.
pub const CORE_UNREACHABLE: i32 = -32099;
const CALL_TIMEOUT: Duration = Duration::from_secs(15);

/// Attesa massima di una risposta del core. Quasi tutto risponde subito; alcune richieste
/// aspettano qualcuno o qualcosa per costruzione e vanno lasciate finire, altrimenti la UI
/// dice «core non raggiungibile» mentre il core sta ancora lavorando (e poi lo stream parte
/// senza finestra che lo accolga).
fn timeout_for(method: &str) -> Duration {
    match method {
        // Presenza dell'amico, condivisione in avvio (30 s) e risposta dell'host al bussare (30 s)
        "session.join" => Duration::from_secs(180),
        // Download e verifica dell'installer
        "update.install" => Duration::from_secs(300),
        // Login di Tailscale alla rete Falco e richieste al Falco Hub
        "net.joinFalco" | "net.loginHeadscale" => Duration::from_secs(90),
        m if m.starts_with("hub.") => Duration::from_secs(30),
        _ => CALL_TIMEOUT,
    }
}

type Pending = HashMap<u64, oneshot::Sender<Result<Value, RpcError>>>;

#[derive(Default)]
pub struct CoreLink {
    writer: Mutex<Option<SendHalf>>,
    pending: Mutex<Pending>,
    next_id: AtomicU64,
    connected: AtomicBool,
}

fn unreachable() -> RpcError {
    RpcError::new(CORE_UNREACHABLE, "core non raggiungibile")
}

impl CoreLink {
    pub fn is_connected(&self) -> bool {
        self.connected.load(Ordering::Relaxed)
    }

    pub async fn call(&self, method: &str, params: Value) -> Result<Value, RpcError> {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed) + 1;
        let line = to_line(&Request::new(id, method, params))
            .map_err(|e| RpcError::new(falco_proto::ipc::codes::INTERNAL, e.to_string()))?;
        let (tx, rx) = oneshot::channel();
        self.pending.lock().await.insert(id, tx);
        {
            let mut w = self.writer.lock().await;
            let Some(writer) = w.as_mut() else {
                self.pending.lock().await.remove(&id);
                return Err(unreachable());
            };
            if writer.write_all(&line).await.is_err() {
                self.pending.lock().await.remove(&id);
                return Err(unreachable());
            }
        }
        match tokio::time::timeout(timeout_for(method), rx).await {
            Ok(Ok(result)) => result,
            _ => {
                self.pending.lock().await.remove(&id);
                Err(unreachable())
            }
        }
    }
}

/// Ciclo di vita del collegamento: connetti, leggi, riconnetti.
/// Avvia il core accanto alla UI se non è in esecuzione (collegamento del menu Start).
fn start_core() {
    let exe = std::env::current_exe().ok().and_then(|p| {
        p.parent().map(|d| d.join(if cfg!(windows) { "falco-core.exe" } else { "falco-core" }))
    });
    if let Some(exe) = exe.filter(|e| e.is_file()) {
        // --autostart: il core non apre un'altra finestra
        let _ = std::process::Command::new(exe).arg("--autostart").spawn();
    }
}

pub async fn run(link: Arc<CoreLink>, app: AppHandle) {
    let mut started = false;
    loop {
        let conn = transport::connect().await;
        if conn.is_err() && !started {
            started = true;
            start_core();
            tokio::time::sleep(Duration::from_millis(800)).await;
            continue;
        }
        if let Ok(stream) = conn {
            let (rx, tx) = transport::split(stream);
            *link.writer.lock().await = Some(tx);
            link.connected.store(true, Ordering::Relaxed);
            let _ = app.emit("core://connected", ());

            let mut lines = BufReader::new(rx).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                match serde_json::from_str::<Incoming>(&line) {
                    Ok(Incoming::Response(r)) => {
                        let id = r.id.as_u64().unwrap_or(0);
                        if let Some(tx) = link.pending.lock().await.remove(&id) {
                            let _ = tx.send(match r.error {
                                Some(e) => Err(e),
                                None => Ok(r.result.unwrap_or(Value::Null)),
                            });
                        }
                    }
                    Ok(Incoming::Notification(n)) => {
                        let _ = app.emit("core://event", n);
                    }
                    Err(_) => {}
                }
            }

            link.connected.store(false, Ordering::Relaxed);
            *link.writer.lock().await = None;
            for (_, tx) in link.pending.lock().await.drain() {
                let _ = tx.send(Err(unreachable()));
            }
            let _ = app.emit("core://disconnected", ());
        }
        tokio::time::sleep(Duration::from_secs(2)).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn long_requests_get_time_to_finish() {
        assert!(timeout_for("session.join") >= Duration::from_secs(150));
        assert!(timeout_for("update.install") > CALL_TIMEOUT);
        assert!(timeout_for("hub.takeInvite") > CALL_TIMEOUT);
        assert_eq!(timeout_for("state.snapshot"), CALL_TIMEOUT);
    }
}
