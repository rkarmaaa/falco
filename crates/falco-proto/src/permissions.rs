//! Ruoli e permessi parziali (§7). Applicati sull'host, nel fork di Sunshine.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    #[default]
    #[serde(rename = "spettatore")]
    Spectator,
    #[serde(rename = "giocatore")]
    Player,
    #[serde(rename = "copilota")]
    Copilot,
    #[serde(rename = "fidato")]
    Trusted,
}

impl Role {
    pub const ALL: [Role; 4] = [Role::Spectator, Role::Player, Role::Copilot, Role::Trusted];

    /// Preset della tabella «Ruoli preimpostati».
    pub fn preset(self) -> Permissions {
        let base = Permissions { role: self, ..Permissions::default() };
        match self {
            Role::Spectator => base,
            Role::Player => Permissions { gamepads: 1, auto_join: true, ..base },
            Role::Copilot => {
                Permissions { gamepads: 2, keyboard: true, mouse: true, auto_join: true, ..base }
            }
            Role::Trusted => Permissions {
                gamepads: 4,
                keyboard: true,
                keyboard_system_keys: true,
                mouse: true,
                auto_join: true,
                launch_apps: true,
                clipboard: true,
                end_host_session: true,
                ..base
            },
        }
    }
}

/// Fascia oraria: giorni (0 = lunedì) e minuti dall'inizio del giorno, fine esclusa.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimeWindow {
    pub days: Vec<u8>,
    pub start_min: u16,
    pub end_min: u16,
}

impl TimeWindow {
    fn contains(&self, weekday: u8, minute: u16) -> bool {
        self.days.contains(&weekday) && minute >= self.start_min && minute < self.end_min
    }
}

/// Permessi effettivi di un amico. Il default nega tutto tranne vedere e sentire.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Permissions {
    pub role: Role,
    pub view: bool,
    pub audio: bool,
    pub gamepads: u8,
    pub keyboard: bool,
    pub keyboard_system_keys: bool,
    pub mouse: bool,
    pub launch_apps: bool,
    pub auto_join: bool,
    pub clipboard: bool,
    pub end_host_session: bool,
    /// Formato `LARGHEZZAxALTEZZA`.
    pub max_resolution: Option<String>,
    pub max_fps: Option<u16>,
    pub max_bitrate_kbps: Option<u32>,
    pub schedule: Vec<TimeWindow>,
    /// Secondi Unix; dopo la scadenza il ruolo torna Spettatore.
    pub expires_at: Option<i64>,
}

impl Default for Permissions {
    fn default() -> Self {
        Self {
            role: Role::Spectator,
            view: true,
            audio: true,
            gamepads: 0,
            keyboard: false,
            keyboard_system_keys: false,
            mouse: false,
            launch_apps: false,
            auto_join: false,
            clipboard: false,
            end_host_session: false,
            max_resolution: None,
            max_fps: None,
            max_bitrate_kbps: None,
            schedule: Vec::new(),
            expires_at: None,
        }
    }
}

/// Modifiche granulari sopra al preset: ogni campo `None` eredita dal ruolo.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct PermissionOverrides {
    pub view: Option<bool>,
    pub audio: Option<bool>,
    pub gamepads: Option<u8>,
    pub keyboard: Option<bool>,
    pub keyboard_system_keys: Option<bool>,
    pub mouse: Option<bool>,
    pub launch_apps: Option<bool>,
    pub auto_join: Option<bool>,
    pub max_resolution: Option<String>,
    pub max_fps: Option<u16>,
    pub max_bitrate_kbps: Option<u32>,
    pub schedule: Option<Vec<TimeWindow>>,
    pub expires_at: Option<i64>,
}

impl Permissions {
    pub fn resolve(role: Role, o: &PermissionOverrides) -> Self {
        let p = role.preset();
        let mut out = Permissions {
            view: o.view.unwrap_or(p.view),
            audio: o.audio.unwrap_or(p.audio),
            gamepads: o.gamepads.unwrap_or(p.gamepads).min(4),
            keyboard: o.keyboard.unwrap_or(p.keyboard),
            keyboard_system_keys: o.keyboard_system_keys.unwrap_or(p.keyboard_system_keys),
            mouse: o.mouse.unwrap_or(p.mouse),
            launch_apps: o.launch_apps.unwrap_or(p.launch_apps),
            auto_join: o.auto_join.unwrap_or(p.auto_join),
            max_resolution: o.max_resolution.clone().or(p.max_resolution.clone()),
            max_fps: o.max_fps.or(p.max_fps),
            max_bitrate_kbps: o.max_bitrate_kbps.or(p.max_bitrate_kbps),
            schedule: o.schedule.clone().unwrap_or_else(|| p.schedule.clone()),
            expires_at: o.expires_at.or(p.expires_at),
            ..p
        };
        // Tasti di sistema senza tastiera non hanno senso
        out.keyboard_system_keys &= out.keyboard;
        out
    }

    /// Permessi validi adesso: scadenza riporta a Spettatore.
    pub fn effective(&self, now: i64) -> Permissions {
        match self.expires_at {
            Some(exp) if now >= exp => Role::Spectator.preset(),
            _ => self.clone(),
        }
    }

    /// Fuori fascia la connessione viene rifiutata. Nessuna fascia = sempre.
    pub fn allowed_at(&self, weekday: u8, minute: u16) -> bool {
        self.schedule.is_empty() || self.schedule.iter().any(|w| w.contains(weekday, minute))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn presets_match_spec_table() {
        let s = Role::Spectator.preset();
        assert!(s.view && s.audio && !s.keyboard && !s.mouse && !s.auto_join && s.gamepads == 0);
        let p = Role::Player.preset();
        assert!(p.gamepads == 1 && p.auto_join && !p.keyboard && !p.mouse);
        let c = Role::Copilot.preset();
        assert!(c.gamepads == 2 && c.keyboard && !c.keyboard_system_keys && c.mouse);
        assert!(!c.launch_apps && !c.clipboard);
        let t = Role::Trusted.preset();
        assert!(t.gamepads == 4 && t.keyboard_system_keys && t.launch_apps && t.end_host_session);
    }

    #[test]
    fn default_denies_everything_but_view() {
        assert_eq!(Permissions::default(), Role::Spectator.preset());
    }

    #[test]
    fn overrides_refine_role() {
        let o = PermissionOverrides { mouse: Some(false), gamepads: Some(9), ..Default::default() };
        let p = Permissions::resolve(Role::Copilot, &o);
        assert!(!p.mouse && p.keyboard);
        assert_eq!(p.gamepads, 4, "massimo 4 controller");
    }

    #[test]
    fn system_keys_need_keyboard() {
        let o = PermissionOverrides { keyboard: Some(false), ..Default::default() };
        assert!(!Permissions::resolve(Role::Trusted, &o).keyboard_system_keys);
    }

    #[test]
    fn expiry_and_schedule() {
        let o = PermissionOverrides {
            expires_at: Some(100),
            schedule: Some(vec![TimeWindow { days: vec![5, 6], start_min: 600, end_min: 1380 }]),
            ..Default::default()
        };
        let p = Permissions::resolve(Role::Trusted, &o);
        assert_eq!(p.effective(99).role, Role::Trusted);
        assert_eq!(p.effective(100), Role::Spectator.preset());
        assert!(p.allowed_at(5, 600));
        assert!(!p.allowed_at(5, 1380));
        assert!(!p.allowed_at(0, 700));
    }

    #[test]
    fn role_wire_names_are_italian() {
        assert_eq!(serde_json::to_string(&Role::Copilot).unwrap(), "\"copilota\"");
    }
}
