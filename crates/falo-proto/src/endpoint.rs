//! Percorsi locali condivisi da core e UI (cartella dati, endpoint IPC).

use std::path::PathBuf;

use directories::ProjectDirs;

/// Cartelle di Falò per l'utente corrente.
pub fn project_dirs() -> Option<ProjectDirs> {
    ProjectDirs::from("app", "Falo", "Falo")
}

/// Nome dell'endpoint IPC del core.
/// Windows: named pipe per utente. macOS: socket Unix nella cartella dati.
pub fn ipc_endpoint() -> IpcEndpoint {
    if cfg!(windows) {
        let user = std::env::var("USERNAME").unwrap_or_else(|_| "user".into());
        IpcEndpoint::Namespaced(format!("falo-core-{user}"))
    } else {
        let dir = project_dirs()
            .map(|d| d.data_local_dir().to_path_buf())
            .unwrap_or_else(std::env::temp_dir);
        IpcEndpoint::Path(dir.join("core.sock"))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IpcEndpoint {
    Namespaced(String),
    Path(PathBuf),
}
