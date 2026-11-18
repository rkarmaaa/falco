//! falco-ui: finestra Tauri. Non parla mai con Tailscale o i motori:
//! inoltra le chiamate JSON-RPC al core e ripubblica i suoi eventi.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod link;

use std::sync::Arc;

use link::CoreLink;
use serde_json::Value;
use tauri::{Manager, State};

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

fn main() {
    let result = tauri::Builder::default()
        .plugin(tauri_plugin_clipboard_manager::init())
        .setup(|app| {
            let link = Arc::new(CoreLink::default());
            app.manage(link.clone());
            tauri::async_runtime::spawn(link::run(link, app.handle().clone()));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            rpc,
            core_connected,
            initial_section,
            initial_link
        ])
        .run(tauri::generate_context!());
    if let Err(e) = result {
        eprintln!("falco-ui: {e}");
        std::process::exit(1);
    }
}
