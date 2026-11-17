//! Tipi condivisi tra core, UI e protocollo di presenza (FPP).
//! Nessuna dipendenza da rete o disco: solo dati, firme e codifiche.
#![cfg_attr(test, allow(clippy::unwrap_used))]

pub mod canonical;
pub mod endpoint;
pub mod identity;
pub mod invite;
pub mod ipc;
pub mod permissions;
pub mod profile;
#[cfg(feature = "transport")]
pub mod transport;

pub use identity::{FaloId, Identity};
pub use permissions::{Permissions, Role};
pub use profile::{Profile, SignedProfile, Status};
