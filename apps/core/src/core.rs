//! Stato del core e orchestrazione. Un solo `Core` condiviso tra IPC e tray.

use std::path::Path;
use std::process::Child;
use std::sync::Arc;

use falo_net::NetworkProvider;
use falo_net::tailscale::Tailscale;
use falo_proto::Status;
use falo_proto::ipc::{NetworkSummary, Notification, StateSnapshot, TailscaleState, events};
use falo_store::config::{Config, Language, Status as CfgStatus};
use falo_store::{ConfigFile, Db};
use serde_json::json;
use tokio::sync::{Mutex, broadcast, mpsc};

pub const MILESTONE: &str = "M0";

/// Comandi dal menu del tray.
#[derive(Debug, Clone)]
pub enum TrayCommand {
    SetStatus(Status),
    SetAvailable(bool),
    OpenUi,
    Quit,
}

/// Quello che il tray deve mostrare.
#[derive(Debug, Clone, PartialEq)]
pub struct TrayUpdate {
    pub language: Language,
    pub status: Status,
    pub host_available: bool,
}

impl TrayUpdate {
    pub fn from_config(c: &Config) -> Self {
        Self {
            language: c.general.language,
            status: from_cfg(c.profile.status),
            host_available: c.host.available_on_start,
        }
    }
}

fn from_cfg(s: CfgStatus) -> Status {
    match s {
        CfgStatus::Online => Status::Online,
        CfgStatus::Busy => Status::Busy,
        CfgStatus::Invisible => Status::Invisible,
    }
}

fn status_str(s: Status) -> &'static str {
    match s {
        Status::Online => "online",
        Status::Busy => "busy",
        Status::Invisible => "invisible",
    }
}

pub type Shared = Arc<Mutex<Core>>;
type Notify = Box<dyn Fn(TrayUpdate) + Send + Sync>;

pub struct Core {
    pub config: ConfigFile,
    pub db: Db,
    pub net: Tailscale,
    pub host_available: bool,
    pub streaming: bool,
    ui: Option<Child>,
    events: broadcast::Sender<Notification>,
    notify_tray: Notify,
}

impl Core {
    pub fn new(config: ConfigFile, db_path: &Path, notify_tray: Notify) -> anyhow::Result<Self> {
        let host_available = config.config().host.available_on_start;
        Ok(Self {
            config,
            db: Db::open(db_path)?,
            net: Tailscale::detect(),
            host_available,
            streaming: false,
            ui: None,
            events: broadcast::channel(64).0,
            notify_tray,
        })
    }

    pub fn subscribe(&self) -> broadcast::Receiver<Notification> {
        self.events.subscribe()
    }

    pub fn emit(&self, method: &str, params: serde_json::Value) {
        // Nessun client connesso non è un errore
        let _ = self.events.send(Notification::new(method, params));
    }

    pub fn status(&self) -> Status {
        from_cfg(self.config.config().profile.status)
    }

    pub fn refresh_tray(&self) {
        (self.notify_tray)(TrayUpdate {
            language: self.config.config().general.language,
            status: self.status(),
            host_available: self.host_available,
        });
    }

    /// Stato della rete; il chiamante non tiene `&Core` durante l'attesa.
    pub async fn net_status(net: Tailscale) -> falo_net::NetStatus {
        net.status().await.unwrap_or_else(|e| {
            tracing::warn!("stato Tailscale non disponibile: {e}");
            falo_net::NetStatus { state: TailscaleState::Stopped, ..falo_net::NetStatus::missing() }
        })
    }

    pub fn snapshot(&self, net: &falo_net::NetStatus) -> StateSnapshot {
        StateSnapshot {
            version: env!("CARGO_PKG_VERSION").into(),
            milestone: MILESTONE.into(),
            status: self.status(),
            host_available: self.host_available,
            streaming: self.streaming,
            network: NetworkSummary {
                provider: "tailscale".into(),
                tailscale: net.state,
                self_ips: net.self_ips.iter().map(ToString::to_string).collect(),
                exit_node_active: net.exit_node_active,
            },
            friends_count: self.db.friend_count().unwrap_or(0),
        }
    }

    pub fn set_status(&mut self, s: Status) -> Result<(), falo_store::ConfigError> {
        self.config.set("profile.status", &json!(status_str(s)))?;
        self.emit(events::PRESENCE_CHANGED, json!({ "self": { "status": s } }));
        self.refresh_tray();
        Ok(())
    }

    /// In M1 qui si avvia o si ferma Sunshine; per ora cambia solo lo stato.
    pub fn set_available(&mut self, available: bool) {
        self.host_available = available;
        self.emit(events::PRESENCE_CHANGED, json!({ "self": { "hostAvailable": available } }));
        self.refresh_tray();
    }

    /// Apre la UI se non è già aperta. La UI è un processo separato (§10).
    pub fn open_ui(&mut self) {
        if let Some(child) = self.ui.as_mut()
            && matches!(child.try_wait(), Ok(None))
        {
            return;
        }
        let exe = std::env::current_exe().ok().and_then(|p| {
            p.parent().map(|d| d.join(if cfg!(windows) { "falo-ui.exe" } else { "falo-ui" }))
        });
        match exe.map(|p| std::process::Command::new(&p).spawn()) {
            Some(Ok(child)) => self.ui = Some(child),
            Some(Err(e)) => tracing::warn!("UI non avviata: {e}"),
            None => tracing::warn!("percorso della UI non trovato"),
        }
    }

    pub fn shutdown(&mut self) {
        if let Some(mut ui) = self.ui.take() {
            let _ = ui.kill();
        }
    }
}

/// Corpo del runtime: server IPC e comandi dal tray.
pub async fn run(core: Core, mut tray_rx: mpsc::UnboundedReceiver<TrayCommand>) {
    let shared: Shared = Arc::new(Mutex::new(core));
    shared.lock().await.refresh_tray();

    let server = tokio::spawn(crate::ipc::serve(shared.clone()));

    while let Some(cmd) = tray_rx.recv().await {
        let mut core = shared.lock().await;
        match cmd {
            TrayCommand::SetStatus(s) => {
                if let Err(e) = core.set_status(s) {
                    tracing::error!("stato non salvato: {e}");
                }
            }
            TrayCommand::SetAvailable(a) => core.set_available(a),
            TrayCommand::OpenUi => core.open_ui(),
            TrayCommand::Quit => {
                core.shutdown();
                break;
            }
        }
    }
    server.abort();
}
