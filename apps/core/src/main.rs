//! falo-core: demone sempre attivo. Tray sul thread principale (tao),
//! IPC e logica su un runtime tokio single-thread separato.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod core;
mod i18n;
mod ipc;
mod tray;

use std::path::PathBuf;

use anyhow::Context;
use falo_proto::ipc::{Request, methods, to_line};
use falo_store::ConfigFile;
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
    pub config: PathBuf,
    pub db: PathBuf,
    pub logs: PathBuf,
}

impl Paths {
    fn resolve() -> anyhow::Result<Self> {
        let dirs = falo_proto::endpoint::project_dirs().context("cartella utente non trovata")?;
        Ok(Self {
            config: dirs.config_dir().join("config.toml"),
            db: dirs.data_dir().join("falo.db"),
            logs: dirs.data_local_dir().join("logs"),
        })
    }
}

fn main() -> anyhow::Result<()> {
    if std::env::args().any(|a| a == "--version") {
        println!("falo-core {}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }

    // Seconda istanza: chiede al core attivo di aprire la UI ed esce
    if forward_to_running_core() {
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
    let notify = {
        let proxy = proxy.clone();
        move |u: TrayUpdate| {
            let _ = proxy.send_event(UserEvent::Update(u));
        }
    };
    let initial = TrayUpdate::from_config(config.config());
    let core = Core::new(config, &paths.db, Box::new(notify))?;

    std::thread::Builder::new().name("falo-runtime".into()).spawn(move || {
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

fn forward_to_running_core() -> bool {
    let Ok(rt) = tokio::runtime::Builder::new_current_thread().enable_all().build() else {
        return false;
    };
    rt.block_on(async {
        let Ok(stream) = falo_proto::transport::connect().await else { return false };
        let (_rx, mut tx) = falo_proto::transport::split(stream);
        let Ok(line) = to_line(&Request::new(1, methods::APP_OPEN_UI, serde_json::Value::Null))
        else {
            return false;
        };
        tx.write_all(&line).await.is_ok()
    })
}

fn init_logging(
    paths: &Paths,
    level: falo_store::config::LogLevel,
) -> Option<tracing_appender::non_blocking::WorkerGuard> {
    use falo_store::config::LogLevel;
    let level = match level {
        LogLevel::Error => "error",
        LogLevel::Warn => "warn",
        LogLevel::Info => "info",
        LogLevel::Debug => "debug",
    };
    std::fs::create_dir_all(&paths.logs).ok()?;
    let appender = tracing_appender::rolling::Builder::new()
        .rotation(tracing_appender::rolling::Rotation::DAILY)
        .max_log_files(7)
        .filename_prefix("core")
        .filename_suffix("log")
        .build(&paths.logs)
        .ok()?;
    let (writer, guard) = tracing_appender::non_blocking(appender);
    let filter = tracing_subscriber::EnvFilter::try_from_env("FALO_LOG")
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(level));
    tracing_subscriber::fmt().with_env_filter(filter).with_writer(writer).with_ansi(false).init();
    Some(guard)
}
