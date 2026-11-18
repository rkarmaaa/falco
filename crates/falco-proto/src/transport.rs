//! Trasporto IPC: named pipe con ACL del solo utente (Windows), socket `0600` (macOS).

use std::io;

use interprocess::local_socket::tokio::Stream;
use interprocess::local_socket::tokio::prelude::*;
use interprocess::local_socket::{
    GenericFilePath, GenericNamespaced, ListenerOptions, Name, ToFsName, ToNsName,
};

use crate::endpoint::{IpcEndpoint, ipc_endpoint};

pub use interprocess::local_socket::tokio::{Listener, Stream as IpcStream};

pub type RecvHalf = tokio::io::ReadHalf<Stream>;
pub type SendHalf = tokio::io::WriteHalf<Stream>;

fn name(ep: &IpcEndpoint) -> io::Result<Name<'static>> {
    match ep {
        IpcEndpoint::Namespaced(s) => s.clone().to_ns_name::<GenericNamespaced>(),
        IpcEndpoint::Path(p) => p.clone().to_fs_name::<GenericFilePath>(),
    }
}

/// Crea il listener del core. Fallisce se un altro core è già in ascolto.
pub fn listen() -> io::Result<Listener> {
    let ep = ipc_endpoint();
    let opts = ListenerOptions::new().name(name(&ep)?);

    #[cfg(windows)]
    let opts = {
        use interprocess::os::windows::local_socket::ListenerOptionsExt;
        use interprocess::os::windows::security_descriptor::SecurityDescriptor;
        // Accesso completo solo a SYSTEM e al proprietario (l'utente che avvia il core)
        let sddl = widestring::U16CString::from_str("D:P(A;;GA;;;SY)(A;;GA;;;OW)")
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;
        opts.security_descriptor(SecurityDescriptor::deserialize(&sddl)?)
    };

    #[cfg(unix)]
    let opts = {
        use interprocess::os::unix::local_socket::ListenerOptionsExt;
        if let IpcEndpoint::Path(p) = &ep
            && let Some(dir) = p.parent()
        {
            std::fs::create_dir_all(dir)?;
        }
        opts.mode(0o600).reclaim_name(true)
    };

    opts.create_tokio()
}

/// Connessione al core in esecuzione.
pub async fn connect() -> io::Result<Stream> {
    Stream::connect(name(&ipc_endpoint())?).await
}

/// Divide uno stream in lettura e scrittura.
pub fn split(stream: Stream) -> (RecvHalf, SendHalf) {
    tokio::io::split(stream)
}
