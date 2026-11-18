//! falco-ui: finestra Tauri. Non parla mai con Tailscale o i motori:
//! inoltra le chiamate JSON-RPC al core e ripubblica i suoi eventi.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod embed;
mod link;

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
    let (w, h) = (480.0, 56.0);
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
            builder.position(x, f64::from(m.position().y) / s + 12.0)
        }
        None => builder.center(),
    };
    builder.build().map(|_| ()).map_err(|e| e.to_string())
}

/// Azione scelta nel pulsantino: la esegue la finestra principale.
#[tauri::command]
fn pill_action(app: tauri::AppHandle, action: String) -> Result<(), String> {
    app.emit_to("main", "pill://action", action).map_err(|e| e.to_string())
}

fn main() {
    let result = tauri::Builder::default()
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
            embed::stream_detach
        ])
        .run(tauri::generate_context!());
    if let Err(e) = result {
        eprintln!("falco-ui: {e}");
        std::process::exit(1);
    }
}
