//! Dove trovare i motori: variabile d'ambiente, pacchetto di Falò, installazione ufficiale.
//! In produzione i motori sono sempre nel pacchetto (ADR 0012).

use std::path::{Path, PathBuf};

fn exe_dir() -> Option<PathBuf> {
    std::env::current_exe().ok()?.parent().map(Path::to_path_buf)
}

fn first_existing(candidates: impl IntoIterator<Item = PathBuf>) -> Option<PathBuf> {
    candidates.into_iter().find(|p| p.is_file())
}

pub fn sunshine() -> Option<PathBuf> {
    if let Some(p) = std::env::var_os("FALO_SUNSHINE_BIN") {
        return Some(PathBuf::from(p));
    }
    let bundled = exe_dir().map(|d| {
        if cfg!(windows) {
            d.join("engines").join("sunshine").join("sunshine.exe")
        } else {
            d.join("../Helpers/falo-sunshine")
        }
    });
    let system: &[&str] = if cfg!(windows) {
        &[r"C:\Program Files\Sunshine\sunshine.exe"]
    } else {
        &["/Applications/Sunshine.app/Contents/MacOS/sunshine", "/opt/homebrew/bin/sunshine"]
    };
    first_existing(bundled.into_iter().chain(system.iter().map(PathBuf::from)))
}

pub fn moonlight() -> Option<PathBuf> {
    if let Some(p) = std::env::var_os("FALO_MOONLIGHT_BIN") {
        return Some(PathBuf::from(p));
    }
    let bundled = exe_dir().map(|d| {
        if cfg!(windows) {
            d.join("engines").join("moonlight").join("Moonlight.exe")
        } else {
            d.join("../Helpers/Moonlight.app/Contents/MacOS/Moonlight")
        }
    });
    let system: &[&str] = if cfg!(windows) {
        &[r"C:\Program Files\Moonlight Game Streaming\Moonlight.exe"]
    } else {
        &["/Applications/Moonlight.app/Contents/MacOS/Moonlight"]
    };
    first_existing(bundled.into_iter().chain(system.iter().map(PathBuf::from)))
}
