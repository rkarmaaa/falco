//! Stato del core e orchestrazione. Un solo `Core` condiviso tra IPC e tray.

use std::path::Path;
use std::process::Child;
use std::sync::Arc;

use falco_net::NetworkProvider;
use falco_net::tailscale::Tailscale;
use falco_proto::Status;
use falco_proto::ipc::{NetworkSummary, Notification, StateSnapshot, TailscaleState, events};
use falco_store::config::{Config, Language, Status as CfgStatus};
use falco_store::{ConfigFile, Db};
use serde_json::json;
use tokio::sync::{Mutex, broadcast, mpsc};

pub const MILESTONE: &str = "M4";

/// Secondi Unix correnti.
pub fn now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// Comandi dal menu del tray.
#[derive(Debug, Clone)]
pub enum TrayCommand {
    SetStatus(Status),
    SetAvailable(bool),
    OpenUi,
    /// Scorciatoia globale: barra rapida dell'host (§7).
    OpenHost,
    /// «Entra» dal menu del tray.
    Join(String),
    Quit,
}

/// Quello che il tray deve mostrare.
#[derive(Debug, Clone, PartialEq)]
pub struct TrayUpdate {
    pub language: Language,
    pub status: Status,
    pub host_available: bool,
    /// Amici in gioco con posti liberi: (ID, nome).
    pub playing: Vec<(String, String)>,
}

impl TrayUpdate {
    pub fn from_config(c: &Config) -> Self {
        Self {
            language: c.general.language,
            status: from_cfg(c.profile.status),
            host_available: c.host.available_on_start,
            playing: Vec::new(),
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
    pub host: crate::host::Host,
    pub streaming: bool,
    pub last_self_ips: Vec<std::net::IpAddr>,
    /// Identità locale; `None` finché l'utente non completa l'onboarding.
    pub me: Option<crate::identity::Me>,
    /// Presenza degli amici, aggiornata dal ciclo di `social::run`.
    pub live: std::collections::HashMap<String, crate::social::Live>,
    /// Finestre collegate all'IPC: decide la frequenza di aggiornamento.
    pub ui_clients: usize,
    /// Sveglia il ciclo di presenza (aggiornamento immediato).
    pub poke: Arc<tokio::sync::Notify>,
    /// Richieste in attesa di approvazione dell'host.
    pub approvals: std::collections::HashMap<String, tokio::sync::oneshot::Sender<bool>>,
    /// Ospiti collegati alla sessione che ospito.
    pub guests: std::collections::HashMap<String, crate::access::ActiveGuest>,
    /// Richieste «bussa» in attesa di risposta.
    pub knocks: std::collections::HashMap<String, tokio::sync::oneshot::Sender<bool>>,
    /// Sessione da ospite in corso: ID dell'amico che ospita.
    pub joined: Option<String>,
    /// «Disponibile» scelto dall'utente (non un avvio temporaneo per il pairing).
    pub user_available: bool,
    ui: Option<Child>,
    events: broadcast::Sender<Notification>,
    notify_tray: Notify,
}

impl Core {
    pub fn new(
        config: ConfigFile,
        db_path: &Path,
        data_dir: &Path,
        notify_tray: Notify,
    ) -> anyhow::Result<Self> {
        let db = Db::open(db_path)?;
        let me = crate::identity::Me::load(&db).unwrap_or_else(|e| {
            tracing::error!("identità non caricata: {e:#}");
            None
        });
        Ok(Self {
            config,
            db,
            net: Tailscale::detect(),
            host_available: false,
            host: crate::host::Host::new(data_dir),
            streaming: false,
            last_self_ips: Vec::new(),
            me,
            live: std::collections::HashMap::new(),
            ui_clients: 0,
            poke: Arc::new(tokio::sync::Notify::new()),
            approvals: std::collections::HashMap::new(),
            guests: std::collections::HashMap::new(),
            knocks: std::collections::HashMap::new(),
            joined: None,
            user_available: false,
            ui: None,
            events: broadcast::channel(64).0,
            notify_tray,
        })
    }

    /// Notifica di sistema (richieste di ingresso, inviti accettati).
    pub fn notify_user(&self, title: &str, body: &str) {
        let r = notify_rust::Notification::new().appname("Falco").summary(title).body(body).show();
        if let Err(e) = r {
            tracing::warn!("notifica non mostrata: {e}");
        }
    }

    pub fn live_remove(&mut self, id: &str) {
        self.live.remove(id);
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
            playing: crate::social::friend_views(self)
                .into_iter()
                .filter(|f| f.state == crate::social::FriendLiveState::Playing)
                .map(|f| (f.id, f.name))
                .collect(),
        });
    }

    /// Stato della rete; il chiamante non tiene `&Core` durante l'attesa.
    pub async fn net_status(net: Tailscale) -> falco_net::NetStatus {
        net.status().await.unwrap_or_else(|e| {
            tracing::warn!("stato Tailscale non disponibile: {e}");
            falco_net::NetStatus {
                state: TailscaleState::Stopped,
                ..falco_net::NetStatus::missing()
            }
        })
    }

    pub fn snapshot(&mut self, net: &falco_net::NetStatus) -> StateSnapshot {
        self.last_self_ips.clone_from(&net.self_ips);
        StateSnapshot {
            host: serde_json::to_value(&self.host.state).unwrap_or_default(),
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
            has_profile: self.me.is_some(),
            oauth_configured: crate::invites::oauth_configured(),
        }
    }

    pub fn set_status(&mut self, s: Status) -> Result<(), falco_store::ConfigError> {
        self.config.set("profile.status", &json!(status_str(s)))?;
        self.emit(events::PRESENCE_CHANGED, json!({ "self": { "status": s } }));
        self.refresh_tray();
        Ok(())
    }

    fn publish_host(&self) {
        self.emit(events::HOST_STATE, serde_json::to_value(&self.host.state).unwrap_or_default());
        self.emit(
            events::PRESENCE_CHANGED,
            json!({ "self": { "hostAvailable": self.host_available } }),
        );
        self.refresh_tray();
    }

    /// Percorso di rete verso un host, per scegliere il bitrate (§8.2).
    pub fn path_to(
        &self,
        address: &str,
        net: &falco_net::NetStatus,
    ) -> falco_engines::moonlight::NetPath {
        use falco_engines::moonlight::NetPath;
        let host = address.rsplit_once(':').map_or(address, |(h, _)| h);
        let Ok(ip) = host.parse::<std::net::IpAddr>() else { return NetPath::Lan };
        let cap = self.config.config().network.relay_bitrate_cap_kbps;
        let relayed = net
            .peers
            .iter()
            .any(|p| p.ips.contains(&ip) && matches!(p.path, falco_net::PeerPath::Relay { .. }));
        match ip {
            _ if relayed => NetPath::Relay { cap_kbps: cap },
            std::net::IpAddr::V4(v4) if v4.is_loopback() || v4.is_private() => NetPath::Lan,
            _ if net.self_ips.contains(&ip) => NetPath::Lan,
            // Banda in upload dell'host ignota fino al test di banda (M5)
            _ => NetPath::Internet { host_upload_kbps: 50_000 },
        }
    }

    /// Chiude la UI prima dello streaming: nessuna webview durante il gioco (§10).
    pub fn close_ui(&mut self) {
        if let Some(mut ui) = self.ui.take() {
            let _ = ui.kill();
        }
    }

    /// Apre la UI se non è già aperta. La UI è un processo separato (§10).
    pub fn open_ui(&mut self) {
        self.open_ui_at(None);
    }

    /// Apre la UI su una sezione; se è già aperta, la porta su quella sezione.
    pub fn open_ui_at(&mut self, section: Option<&str>) {
        if let Some(child) = self.ui.as_mut()
            && matches!(child.try_wait(), Ok(None))
        {
            if let Some(s) = section {
                self.emit(events::UI_NAVIGATE, json!({ "section": s }));
            }
            return;
        }
        let exe = std::env::current_exe().ok().and_then(|p| {
            p.parent().map(|d| d.join(if cfg!(windows) { "falco-ui.exe" } else { "falco-ui" }))
        });
        let args: Vec<String> =
            section.map(|s| vec!["--section".into(), s.into()]).unwrap_or_default();
        match exe.map(|p| std::process::Command::new(&p).args(&args).spawn()) {
            Some(Ok(child)) => self.ui = Some(child),
            Some(Err(e)) => tracing::warn!("UI non avviata: {e}"),
            None => tracing::warn!("percorso della UI non trovato"),
        }
    }

    pub async fn shutdown(&mut self) {
        self.close_ui();
        self.host.stop().await;
    }
}

/// «Disponibile»: avvia o ferma Sunshine e segue l'avvio fino a «pronto».
pub async fn set_available(shared: Shared, available: bool) {
    let mut c = shared.lock().await;
    c.host_available = available;
    if !available {
        c.host.stop().await;
        c.publish_host();
        return;
    }
    let cfg = c.config.config().clone();
    if let Err(e) = c.host.start(&cfg).await {
        tracing::warn!("host non avviato: {e:#}");
        c.host.state = crate::host::HostState::Error(format!("{e:#}"));
        c.host_available = false;
        c.publish_host();
        return;
    }
    c.publish_host();
    let Some(api) = c.host.api_any() else { return };
    drop(c);
    let ready = crate::host::Host::wait_ready(api).await;
    let mut c = shared.lock().await;
    if c.host_available {
        let alive = c.host.alive();
        c.guests.clear();
        c.host.state = if ready && alive {
            crate::host::HostState::Ready
        } else {
            crate::host::HostState::Error("il motore host non risponde".into())
        };
        c.publish_host();
        drop(c);
        // Client degli amici allineati ai permessi appena l'host è pronto
        crate::access::apply_all(&shared).await;
    }
}

/// Corpo del runtime: server IPC e comandi dal tray.
pub async fn run(core: Core, mut tray_rx: mpsc::UnboundedReceiver<TrayCommand>) {
    let shared: Shared = Arc::new(Mutex::new(core));
    shared.lock().await.refresh_tray();

    let server = tokio::spawn(crate::ipc::serve(shared.clone()));
    let presence_server = tokio::spawn(crate::fpp_server::run(shared.clone()));
    let presence_client = tokio::spawn(crate::social::run(shared.clone()));
    let access = tokio::spawn(crate::access::run(shared.clone()));
    let available = shared.lock().await.config.config().host.available_on_start;
    if available {
        tokio::spawn(set_available(shared.clone(), true));
    }

    while let Some(cmd) = tray_rx.recv().await {
        if let TrayCommand::Join(id) = &cmd {
            let (s, id) = (shared.clone(), id.clone());
            tokio::spawn(async move {
                if let Err(e) = crate::ipc::join_friend(&s, &id, "Desktop").await {
                    s.lock().await.notify_user("Non sono riuscito a entrare", &e.message);
                }
            });
            continue;
        }
        if let TrayCommand::SetAvailable(a) = cmd {
            shared.lock().await.user_available = a;
            tokio::spawn(set_available(shared.clone(), a));
            continue;
        }
        let mut core = shared.lock().await;
        match cmd {
            TrayCommand::SetStatus(s) => {
                if let Err(e) = core.set_status(s) {
                    tracing::error!("stato non salvato: {e}");
                }
            }
            TrayCommand::OpenUi => core.open_ui(),
            TrayCommand::OpenHost => core.open_ui_at(Some("host")),
            TrayCommand::Quit => {
                core.shutdown().await;
                break;
            }
            TrayCommand::SetAvailable(_) | TrayCommand::Join(_) => {}
        }
    }
    server.abort();
    presence_server.abort();
    presence_client.abort();
    access.abort();
}
