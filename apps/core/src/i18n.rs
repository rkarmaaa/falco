//! Testi del tray. La UI ha il proprio modulo i18n; qui bastano poche voci.

use falo_store::config::Language;

pub struct TrayText {
    pub tooltip: &'static str,
    pub status: &'static str,
    pub online: &'static str,
    pub busy: &'static str,
    pub invisible: &'static str,
    pub available: &'static str,
    pub open: &'static str,
    pub quit: &'static str,
}

pub fn tray(lang: Language) -> TrayText {
    match lang {
        Language::It => TrayText {
            tooltip: "Falò",
            status: "Stato",
            online: "Online",
            busy: "Occupato",
            invisible: "Invisibile",
            available: "Disponibile per gli amici",
            open: "Apri Falò",
            quit: "Esci",
        },
        Language::En => TrayText {
            tooltip: "Falò",
            status: "Status",
            online: "Online",
            busy: "Busy",
            invisible: "Invisible",
            available: "Available to friends",
            open: "Open Falò",
            quit: "Quit",
        },
    }
}
