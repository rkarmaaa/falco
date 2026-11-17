//! Dottore di rete (§9): ogni controllo produce un semaforo e, se serve, un'azione.
//! In M0 solo i controlli locali su Tailscale; gli altri arrivano in M5.

use serde::Serialize;

use crate::{NetStatus, PeerPath, TailscaleState};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Level {
    Ok,
    Warn,
    Fail,
    /// Controllo previsto ma non ancora disponibile.
    Pending,
}

/// Un controllo. Testi in UI tramite chiavi i18n `doctor.<id>.<level>`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Check {
    pub id: &'static str,
    pub level: Level,
    pub value: Option<String>,
    /// Azione suggerita, chiave i18n `doctor.action.<id>`.
    pub action: Option<&'static str>,
}

impl Check {
    fn new(id: &'static str, level: Level) -> Self {
        Self { id, level, value: None, action: None }
    }
    fn value(mut self, v: impl Into<String>) -> Self {
        self.value = Some(v.into());
        self
    }
    fn action(mut self, a: &'static str) -> Self {
        self.action = Some(a);
        self
    }
}

/// Controlli derivati dallo stato di Tailscale.
pub fn tailscale_checks(status: &NetStatus, warn_exit_node: bool) -> Vec<Check> {
    let mut out = Vec::new();
    let installed = match status.state {
        TailscaleState::Missing => {
            Check::new("tailscaleInstalled", Level::Fail).action("installTailscale")
        }
        _ => Check::new("tailscaleInstalled", Level::Ok),
    };
    out.push(installed);

    let running = match status.state {
        TailscaleState::Running => Check::new("tailscaleRunning", Level::Ok)
            .value(status.self_ips.first().map(ToString::to_string).unwrap_or_default()),
        TailscaleState::NeedsLogin => {
            Check::new("tailscaleRunning", Level::Warn).action("loginTailscale")
        }
        TailscaleState::Stopped => {
            Check::new("tailscaleRunning", Level::Fail).action("startTailscale")
        }
        TailscaleState::Missing => Check::new("tailscaleRunning", Level::Pending),
    };
    out.push(running);

    if warn_exit_node && status.exit_node_active {
        out.push(Check::new("exitNode", Level::Warn).action("disableExitNode"));
    }

    let relayed = status.peers.iter().filter(|p| matches!(p.path, PeerPath::Relay { .. })).count();
    if relayed > 0 {
        out.push(
            Check::new("peersRelay", Level::Warn).value(relayed.to_string()).action("enableUpnp"),
        );
    }

    for id in ["udp", "sunshinePorts", "bandwidth", "jitter", "localSetup"] {
        out.push(Check::new(id, Level::Pending));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_tailscale_fails_with_action() {
        let checks = tailscale_checks(&NetStatus::missing(), true);
        assert_eq!(checks[0].level, Level::Fail);
        assert_eq!(checks[0].action, Some("installTailscale"));
    }

    #[test]
    fn relay_and_exit_node_warn() {
        let json = include_str!("../tests/fixtures/status_running.json");
        let mut s = crate::tailscale::parse_status(json.as_bytes()).unwrap();
        s.exit_node_active = true;
        let checks = tailscale_checks(&s, true);
        assert!(checks.iter().any(|c| c.id == "exitNode" && c.level == Level::Warn));
        assert!(checks.iter().any(|c| c.id == "peersRelay" && c.value.as_deref() == Some("1")));
        assert!(!tailscale_checks(&s, false).iter().any(|c| c.id == "exitNode"));
    }
}
