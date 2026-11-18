//! Percorsi locali condivisi da core e UI (cartella dati, endpoint IPC).

use std::path::PathBuf;

use directories::ProjectDirs;

/// Istanza separata per sviluppo e test (`FALO_INSTANCE=b`): cartelle e IPC propri.
pub fn instance() -> Option<String> {
    std::env::var("FALO_INSTANCE").ok().filter(|s| !s.is_empty())
}

/// Cartelle di Falò per l'utente corrente.
pub fn project_dirs() -> Option<ProjectDirs> {
    match instance() {
        Some(i) => ProjectDirs::from("app", "Falo", &format!("Falo-{i}")),
        None => ProjectDirs::from("app", "Falo", "Falo"),
    }
}

/// Nome dell'endpoint IPC del core.
/// Windows: named pipe per utente. macOS: socket Unix nella cartella dati.
pub fn ipc_endpoint() -> IpcEndpoint {
    if cfg!(windows) {
        let user = std::env::var("USERNAME").unwrap_or_else(|_| "user".into());
        let suffix = instance().map(|i| format!("-{i}")).unwrap_or_default();
        IpcEndpoint::Namespaced(format!("falo-core-{user}{suffix}"))
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
