//! Motori di streaming (§8): Falò genera configurazioni e legge lo stato,
//! non tocca mai i pacchetti video.
#![cfg_attr(test, allow(clippy::unwrap_used))]

pub mod locate;
pub mod moonlight;
pub mod process;
pub mod sunshine;
pub mod sunshine_api;
