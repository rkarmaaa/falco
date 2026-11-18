//! falco-core: demone sempre attivo. Tray sul thread principale (tao),
//! IPC e logica su un runtime tokio single-thread separato.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#![cfg_attr(test, allow(clippy::unwrap_used))]

mod access;
mod bench;
mod core;
mod debug;
mod doctor;
mod fpp_server;
mod guest;
mod host;
mod hub;
mod i18n;
mod identity;
mod input_monitor;
mod invites;
mod ipc;
mod netmodes;
mod platform;
mod secrets;
mod social;
mod tray;
mod updater;

use std::path::PathBuf;

use anyhow::Context;
use falco_proto::ipc::{Request, methods, to_line};
use falco_store::ConfigFile;
use tao::event::{Event, StartCause};
use tao::event_loop::{ControlFlow, EventLoopBuilder};
use tokio::io::AsyncWriteExt;
use tokio::sync::mpsc;

use crate::core::{Core, TrayCommand, TrayUpdate};

/// Evento del loop principale: menu del tray o aggiornamento dal core.
#[derive(Debug)]
pub enum UserEvent {
    Menu(tray_icon::menu::MenuEvent),
    Update(TrayUpdate),
}

pub struct Paths {
    pub data: PathBuf,
    pub config: PathBuf,
    pub db: PathBuf,
    pub logs: PathBuf,
}

impl Paths {
    fn resolve() -> anyhow::Result<Self> {
        let dirs = falco_proto::endpoint::project_dirs().context("cartella utente non trovata")?;
        Ok(Self {
            data: dirs.data_local_dir().to_path_buf(),
            config: dirs.config_dir().join("config.toml"),
            db: dirs.data_dir().join("falco.db"),
            logs: dirs.data_local_dir().join("logs"),
        })
    }
}

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == "--version") {
        println!("falco-core {}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }
    if args.get(1).map(String::as_str) == Some("bench") {
        return bench::run(&args[2..]);
    }
    // Diagnostica: app viste da Moonlight su un host (`falco-core debug-list ip:porta`)
    if args.get(1).map(String::as_str) == Some("debug-list") {
        let bin = falco_engines::locate::moonlight().context("Moonlight non trovato")?;
        let addr = args.get(2).cloned().unwrap_or_default();
        let rt = tokio::runtime::Builder::new_current_thread().enable_all().build()?;
        println!("{:?}", rt.block_on(guest::list_apps(&bin, &addr)));
        return Ok(());
    }

    // Link falco:// aperto dal sistema (§14)
    let link = args.iter().position(|a| a == "--open-link").and_then(|i| args.get(i + 1).cloned());
    let autostart = args.iter().any(|a| a == "--autostart");

    // Seconda istanza: chiede al core attivo di aprire la UI (o il link) ed esce
    if forward_to_running_core(link.as_deref()) {
        return Ok(());
    }

    let paths = Paths::resolve()?;
    let config = ConfigFile::load_or_create(&paths.config)
        .with_context(|| format!("config non valida: {}", paths.config.display()))?;
    let _log_guard = init_logging(&paths, config.config().advanced.log_level);

    let event_loop = EventLoopBuilder::<UserEvent>::with_user_event().build();
    let proxy = event_loop.create_proxy();

    let menu_proxy = proxy.clone();
    tray_icon::menu::MenuEvent::set_event_handler(Some(move |e| {
        let _ = menu_proxy.send_event(UserEvent::Menu(e));
    }));

    let (cmd_tx, cmd_rx) = mpsc::unbounded_channel::<TrayCommand>();

    // Barra rapida dell'host: Ctrl+Shift+F (Cmd+Shift+F su macOS), §7
    let _hotkeys = register_hotkey(cmd_tx.clone());
    let notify = {
        let proxy = proxy.clone();
        move |u: TrayUpdate| {
            let _ = proxy.send_event(UserEvent::Update(u));
        }
    };
    let initial = TrayUpdate::from_config(config.config());
    // Integrazione di sistema solo nelle build installate, non da `cargo run`
    if !cfg!(debug_assertions) {
        if let Err(e) = platform::sync_autostart(config.config().general.start_with_system) {
            tracing::warn!("avvio con il sistema: {e:#}");
        }
        if let Err(e) = platform::register_url_scheme() {
            tracing::warn!("schema falco://: {e:#}");
        }
    }
    let open_at_start = link.is_none() && !autostart && !config.config().general.start_minimized;
    let core = Core::new(config, &paths.db, &paths.data, Box::new(notify))?;
    let mut core = core;
    // Avvio da menu Start o Launchpad: si apre la finestra; con --autostart resta nel tray
    if open_at_start {
        core.open_ui();
    }
    if let Some(l) = &link {
        core.open_link(l);
    }

    std::thread::Builder::new().name("falco-runtime".into()).spawn(move || {
        let rt = match tokio::runtime::Builder::new_current_thread().enable_all().build() {
            Ok(rt) => rt,
            Err(e) => {
                tracing::error!("runtime non avviato: {e}");
                return;
            }
        };
        rt.block_on(core::run(core, cmd_rx));
    })?;

    let mut tray: Option<tray::Tray> = None;
    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;
        match event {
            // Su macOS il tray va creato dopo l'avvio del loop
            Event::NewEvents(StartCause::Init) => match tray::Tray::build(&initial) {
                Ok(t) => tray = Some(t),
                Err(e) => tracing::error!("tray non creato: {e:#}"),
            },
            Event::UserEvent(UserEvent::Menu(e)) => {
                if let Some(cmd) = tray.as_ref().and_then(|t| t.command_for(&e)) {
                    let quit = matches!(cmd, TrayCommand::Quit);
                    let _ = cmd_tx.send(cmd);
                    if quit {
                        tray.take();
                        *control_flow = ControlFlow::Exit;
                    }
                }
            }
            Event::UserEvent(UserEvent::Update(u)) => {
                if let Some(t) = tray.as_mut() {
                    t.apply(&u);
                }
            }
            _ => {}
        }
    });
}

fn register_hotkey(
    tx: mpsc::UnboundedSender<TrayCommand>,
) -> Option<global_hotkey::GlobalHotKeyManager> {
    use global_hotkey::hotkey::{Code, HotKey, Modifiers};
    use global_hotkey::{GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState};
    let manager = GlobalHotKeyManager::new().ok()?;
    let mods = if cfg!(target_os = "macos") { Modifiers::SUPER } else { Modifiers::CONTROL };
    let key = HotKey::new(Some(mods | Modifiers::SHIFT), Code::KeyF);
    if let Err(e) = manager.register(key) {
        tracing::warn!("scorciatoia non registrata: {e}");
        return None;
    }
    let id = key.id();
    GlobalHotKeyEvent::set_event_handler(Some(move |e: GlobalHotKeyEvent| {
        if e.id == id && e.state == HotKeyState::Pressed {
            let _ = tx.send(TrayCommand::OpenHost);
        }
    }));
    Some(manager)
}

fn forward_to_running_core(link: Option<&str>) -> bool {
    let Ok(rt) = tokio::runtime::Builder::new_current_thread().enable_all().build() else {
        return false;
    };
    rt.block_on(async {
        let Ok(stream) = falco_proto::transport::connect().await else { return false };
        let (_rx, mut tx) = falco_proto::transport::split(stream);
        let req = match link {
            Some(l) => Request::new(1, methods::APP_OPEN_LINK, serde_json::json!({ "link": l })),
            None => Request::new(1, methods::APP_OPEN_UI, serde_json::Value::Null),
        };
        let Ok(line) = to_line(&req) else {
            return false;
        };
        tx.write_all(&line).await.is_ok()
    })
}

fn init_logging(
    paths: &Paths,
    level: falco_store::config::LogLevel,
) -> Option<tracing_appender::non_blocking::WorkerGuard> {
    use falco_store::config::LogLevel;
    let level = match level {
        LogLevel::Error => "error",
        LogLevel::Warn => "warn",
        LogLevel::Info => "info",
        LogLevel::Debug => "debug",
    };
    std::fs::create_dir_all(&paths.logs).ok()?;
    let appender = match tracing_appender::rolling::Builder::new()
        .rotation(tracing_appender::rolling::Rotation::DAILY)
        .max_log_files(7)
        .filename_prefix("core")
        .filename_suffix("log")
        .build(&paths.logs)
    {
        Ok(a) => a,
        Err(e) => {
            eprintln!("log non disponibili: {e}");
            return None;
        }
    };
    let (writer, guard) = tracing_appender::non_blocking(appender);
    let filter = tracing_subscriber::EnvFilter::try_from_env("FALCO_LOG")
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(level));
    tracing_subscriber::fmt().with_env_filter(filter).with_writer(writer).with_ansi(false).init();
    Some(guard)
}
