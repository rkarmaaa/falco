//! falco-ui: finestra Tauri. Non parla mai con Tailscale o i motori:
//! inoltra le chiamate JSON-RPC al core e ripubblica i suoi eventi.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod embed;
mod link;
mod volume;

use std::sync::Arc;

use link::CoreLink;
use serde_json::Value;
use tauri::{Emitter, Manager, State, WebviewUrl, WebviewWindowBuilder};

/// Chiamata JSON-RPC verso il core.
#[tauri::command]
async fn rpc(
    link: State<'_, Arc<CoreLink>>,
    method: String,
    params: Option<Value>,
) -> Result<Value, falco_proto::ipc::RpcError> {
    link.call(&method, params.unwrap_or(Value::Null)).await
}

fn arg(name: &str) -> Option<String> {
    let args: Vec<String> = std::env::args().collect();
    args.iter().position(|a| a == name).and_then(|i| args.get(i + 1).cloned())
}

/// Sezione richiesta all'avvio (`--section host` dalla scorciatoia del core).
#[tauri::command]
fn initial_section() -> Option<String> {
    arg("--section")
}

/// Link di invito con cui è stata aperta la finestra (`falco://invite/…`).
#[tauri::command]
fn initial_link() -> Option<String> {
    arg("--link")
}

/// Il core è raggiungibile in questo momento?
#[tauri::command]
fn core_connected(link: State<'_, Arc<CoreLink>>) -> bool {
    link.is_connected()
}

/// Pulsantino sopra il flusso a schermo intero: finestra piccola, trasparente, sempre in primo piano.
/// Asincrono: su Windows creare una finestra da un comando sincrono la blocca.
#[tauri::command]
async fn pill(app: tauri::AppHandle, show: bool) -> Result<(), String> {
    if let Some(w) = app.get_webview_window("pill") {
        if !show {
            return w.close().map_err(|e| e.to_string());
        }
        return Ok(());
    }
    if !show {
        return Ok(());
    }
    let main = app.get_webview_window("main").ok_or("finestra principale assente")?;
    let monitor = main.current_monitor().map_err(|e| e.to_string())?;
    let (w, h) = (PILL_W, PILL_H);
    let builder =
        WebviewWindowBuilder::new(&app, "pill", WebviewUrl::App("index.html#pill".into()))
            .title("Falco")
            .inner_size(w, h)
            .decorations(false)
            .transparent(true)
            .shadow(false)
            .resizable(false)
            .always_on_top(true)
            .skip_taskbar(true)
            .focused(false);
    let builder = match monitor {
        Some(m) => {
            let s = m.scale_factor();
            let x = f64::from(m.position().x) / s + (f64::from(m.size().width) / s - w) / 2.0;
            builder.position(x, f64::from(m.position().y) / s)
        }
        None => builder.center(),
    };
    builder.build().map(|_| ()).map_err(|e| e.to_string())
}

const PILL_W: f64 = 480.0;
/// Pulsantino visibile (con 12 px di margine dal bordo) e nascosto: una striscia di pochi pixel
/// sul bordo alto, che lo fa ricomparire quando ci arriva il cursore senza coprire lo stream.
const PILL_H: f64 = 68.0;
const PILL_STRIP_H: f64 = 6.0;

/// Nasconde o mostra il pulsantino ridimensionando la sua finestra.
#[tauri::command]
fn pill_collapse(app: tauri::AppHandle, collapsed: bool) -> Result<(), String> {
    let w = app.get_webview_window("pill").ok_or("pulsantino assente")?;
    let h = if collapsed { PILL_STRIP_H } else { PILL_H };
    w.set_size(tauri::LogicalSize::new(PILL_W, h)).map_err(|e| e.to_string())
}

/// Finestra delle notifiche sopra lo stream (coordinate logiche dello schermo): trasparente,
/// sempre in primo piano, grande quanto le notifiche. `show: false` la chiude.
#[tauri::command]
async fn toasts_window(
    app: tauri::AppHandle,
    show: bool,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
) -> Result<(), String> {
    let existing = app.get_webview_window("toasts");
    if !show {
        if let Some(win) = existing {
            win.close().map_err(|e| e.to_string())?;
        }
        return Ok(());
    }
    if let Some(win) = existing {
        win.set_size(tauri::LogicalSize::new(w, h)).map_err(|e| e.to_string())?;
        return win.set_position(tauri::LogicalPosition::new(x, y)).map_err(|e| e.to_string());
    }
    WebviewWindowBuilder::new(&app, "toasts", WebviewUrl::App("index.html#toasts".into()))
        .title("Falco")
        .inner_size(w, h)
        .position(x, y)
        .decorations(false)
        .transparent(true)
        .shadow(false)
        .resizable(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .focused(false)
        .build()
        .map(|_| ())
        .map_err(|e| e.to_string())
}

/// Azione scelta nel pulsantino: la esegue la finestra principale.
#[tauri::command]
fn pill_action(app: tauri::AppHandle, action: String) -> Result<(), String> {
    app.emit_to("main", "pill://action", action).map_err(|e| e.to_string())
}

fn main() {
    let result = tauri::Builder::default()
        // Per primo: una seconda istanza passa i suoi argomenti a questa e si chiude
        .plugin(tauri_plugin_single_instance::init(|app, args, _cwd| {
            if let Some(w) = app.get_webview_window("main") {
                let _ = w.unminimize();
                let _ = w.show();
                let _ = w.set_focus();
            }
            let value = |name: &str| {
                args.iter().position(|a| a == name).and_then(|i| args.get(i + 1)).cloned()
            };
            if let Some(link) = value("--link") {
                let _ = app.emit("ui://link", link);
            }
            if let Some(section) = value("--section") {
                let _ = app.emit("ui://section", section);
            }
        }))
        .plugin(tauri_plugin_clipboard_manager::init())
        .setup(|app| {
            let link = Arc::new(CoreLink::default());
            app.manage(link.clone());
            app.manage(embed::Embedded::default());
            tauri::async_runtime::spawn(link::run(link, app.handle().clone()));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            rpc,
            core_connected,
            initial_section,
            initial_link,
            pill,
            pill_action,
            embed::stream_attach,
            embed::stream_place,
            embed::stream_detach,
            embed::stream_focus,
            pill_collapse,
            toasts_window,
            embed::stream_stats,
            volume::stream_volume
        ])
        .run(tauri::generate_context!());
    if let Err(e) = result {
        eprintln!("falco-ui: {e}");
        std::process::exit(1);
    }
}
