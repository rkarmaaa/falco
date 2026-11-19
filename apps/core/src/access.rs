//! Permessi lato host (§7) applicabili con Sunshine ufficiale: accesso, «bussa»,
//! fasce orarie, scadenza, posti. Il filtro degli input richiede il fork (ADR 0011).

use std::time::Duration;

use falco_proto::Permissions;
use falco_proto::ipc::events;
use falco_proto::permissions::PermissionOverrides;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::sync::oneshot;

use crate::core::{Core, Shared, now};

/// Ospite collegato (sessione approvata).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ActiveGuest {
    pub friend_id: String,
    pub name: String,
    pub since: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", tag = "result")]
pub enum KnockAnswer {
    Accepted { permissions: Box<Permissions> },
    Declined { reason: String },
}

/// Permessi effettivi di un amico adesso (ruolo + override, scadenza).
pub fn effective(c: &Core, friend_id: &str) -> Option<Permissions> {
    let (role, overrides, _) = c.db.permissions(friend_id).ok().flatten()?;
    let role = serde_json::from_value(json!(role)).ok()?;
    let o: PermissionOverrides = serde_json::from_str(&overrides).unwrap_or_default();
    Some(Permissions::resolve(role, &o).effective(now()))
}

/// Giorno della settimana (0 = lunedì) e minuto locale.
fn local_time() -> (u8, u16) {
    let secs = now() + local_offset_s();
    let days = secs.div_euclid(86_400);
    let minute = (secs.rem_euclid(86_400) / 60) as u16;
    // 1970-01-01 era giovedì (3 se lunedì = 0)
    (((days + 3).rem_euclid(7)) as u8, minute)
}

fn local_offset_s() -> i64 {
    // Scostamento dall'UTC letto dal sistema; 0 se non disponibile
    std::env::var("FALCO_TZ_OFFSET_MIN")
        .ok()
        .and_then(|v| v.parse::<i64>().ok())
        .map(|m| m * 60)
        .unwrap_or_else(system_offset_s)
}

#[cfg(windows)]
fn system_offset_s() -> i64 {
    let out = std::process::Command::new("powershell")
        .args([
            "-NoProfile",
            "-Command",
            "[int][TimeZoneInfo]::Local.GetUtcOffset([DateTime]::Now).TotalMinutes",
        ])
        .output();
    out.ok()
        .and_then(|o| String::from_utf8_lossy(&o.stdout).trim().parse::<i64>().ok())
        .map(|m| m * 60)
        .unwrap_or(0)
}

#[cfg(not(windows))]
fn system_offset_s() -> i64 {
    let out = std::process::Command::new("date").arg("+%z").output();
    out.ok()
        .and_then(|o| {
            let s = String::from_utf8_lossy(&o.stdout).trim().to_owned();
            let sign = if s.starts_with('-') { -1 } else { 1 };
            let n: i64 = s.trim_start_matches(['+', '-']).parse().ok()?;
            Some(sign * ((n / 100) * 3600 + (n % 100) * 60))
        })
        .unwrap_or(0)
}

/// Dispositivi che almeno un amico può usare: gli altri restano spenti in Sunshine.
/// Default: nessuno, cioè ogni ospite può solo guardare (ADR 0015).
pub fn wanted_inputs(c: &Core) -> falco_engines::sunshine::Inputs {
    let mut i = falco_engines::sunshine::Inputs::default();
    for f in c.db.friends().unwrap_or_default() {
        if let Some(p) = effective(c, &f.id) {
            i.keyboard |= p.keyboard;
            i.mouse |= p.mouse;
            i.controller |= p.gamepads > 0;
        }
    }
    i
}

/// Riavvia Sunshine se gli input da accendere sono cambiati; con ospiti collegati aspetta la fine.
pub async fn sync_inputs(shared: &Shared) {
    let (restart, pending) = {
        let mut c = shared.lock().await;
        let wanted = wanted_inputs(&c);
        let running = c.host.api().is_some();
        let differs = running && wanted != c.host.inputs;
        // Ospiti collegati o un'associazione in corso: il riavvio aspetta
        let busy = !c.guests.is_empty() || crate::invites::pairing_active();
        c.inputs_pending = differs && busy;
        (differs && !busy, c.inputs_pending)
    };
    if pending {
        shared
            .lock()
            .await
            .emit(events::HOST_STATE, json!({ "state": "ready", "message": "inputs-pending" }));
    }
    if restart {
        let user = shared.lock().await.user_available;
        crate::core::set_available(shared.clone(), false).await;
        crate::core::set_available(shared.clone(), true).await;
        shared.lock().await.user_available = user;
    }
}

/// Il client Sunshine di un amico deve essere abilitato adesso?
fn should_enable(c: &Core, friend_id: &str, p: &Permissions) -> bool {
    let (day, minute) = local_time();
    p.view && p.allowed_at(day, minute) && (p.auto_join || c.guests.contains_key(friend_id))
}

/// Allinea Sunshine ai permessi: abilita o blocca ogni client associato.
pub async fn apply_all(shared: &Shared) {
    let (api, plan) = {
        let c = shared.lock().await;
        let Some(api) = c.host.api() else { return };
        let friends = c.db.friends().unwrap_or_default();
        // (amico, nome, uuid noto, abilitare?)
        let plan: Vec<(String, String, Option<String>, bool)> = friends
            .iter()
            .filter_map(|f| {
                let uuid = c.db.permissions(&f.id).ok().flatten()?.2;
                let p = effective(&c, &f.id)?;
                Some((f.id.clone(), f.name.clone(), uuid, should_enable(&c, &f.id, &p)))
            })
            .collect();
        (api, plan)
    };
    let learned = tokio::task::spawn_blocking(move || {
        let current = api.clients().unwrap_or_default();
        let mut learned = Vec::new();
        for (fid, name, uuid, enabled) in plan {
            // Ripiego: client associato con il nome dell'amico ma uuid non registrato
            let uuid = uuid.or_else(|| {
                let u = current.iter().find(|c| c.name == name).map(|c| c.uuid.clone())?;
                learned.push((fid, u.clone()));
                Some(u)
            });
            if let Some(uuid) = uuid
                && current.iter().any(|c| c.uuid == uuid && c.enabled != enabled)
            {
                let _ = api.set_enabled(&uuid, enabled);
            }
        }
        learned
    })
    .await
    .unwrap_or_default();
    if !learned.is_empty() {
        let c = shared.lock().await;
        for (fid, uuid) in learned {
            let _ = c.db.set_client_cert(&fid, &uuid);
        }
    }
}

/// Un amico bussa (§7 «Bussare»): controlli dell'host e, se serve, approvazione.
pub async fn knock(shared: &Shared, friend_id: &str) -> KnockAnswer {
    let declined = |r: &str| KnockAnswer::Declined { reason: r.to_owned() };
    let (perms, name, full, timeout) = {
        let c = shared.lock().await;
        if c.host.api().is_none() {
            return declined("not-hosting");
        }
        let Some(p) = effective(&c, friend_id) else { return declined("unknown") };
        let name = c.db.friend(friend_id).ok().flatten().map(|f| f.name).unwrap_or_default();
        let cfg = c.config.config();
        (p, name, c.guests.len() >= usize::from(cfg.host.max_guests), cfg.host.knock_timeout_s)
    };
    if !perms.view {
        return declined("no-access");
    }
    let (day, minute) = local_time();
    if !perms.allowed_at(day, minute) {
        return declined("schedule");
    }
    if full {
        return declined("full");
    }
    if !perms.auto_join {
        let (tx, rx) = oneshot::channel();
        {
            let mut c = shared.lock().await;
            c.knocks.insert(friend_id.to_owned(), tx);
            c.emit(events::SESSION_KNOCK, json!({ "friendId": friend_id, "name": name }));
            c.notify_user(
                &format!("{name} vuole entrare"),
                "Apri Falco per accettare o rifiutare.",
            );
        }
        let answer = tokio::time::timeout(Duration::from_secs(u64::from(timeout)), rx).await;
        shared.lock().await.knocks.remove(friend_id);
        match answer {
            Ok(Ok(true)) => {}
            Ok(Ok(false)) => return declined("declined"),
            _ => return declined("no-answer"),
        }
    }
    {
        let mut c = shared.lock().await;
        c.guests.insert(
            friend_id.to_owned(),
            ActiveGuest { friend_id: friend_id.to_owned(), name, since: now() },
        );
        emit_guests(&mut c);
    }
    apply_all(shared).await;
    KnockAnswer::Accepted { permissions: Box::new(perms) }
}

/// L'ospite ha lasciato la sessione: chi non entra da solo torna bloccato.
pub async fn leave(shared: &Shared, friend_id: &str) {
    {
        let mut c = shared.lock().await;
        c.guests.remove(friend_id);
        emit_guests(&mut c);
    }
    apply_all(shared).await;
    // Permessi cambiati durante la sessione: si applicano ora che è finita
    if shared.lock().await.inputs_pending {
        sync_inputs(shared).await;
    }
}

/// «Espelli»: blocco immediato in Sunshine e fine della sua sessione.
pub async fn kick(shared: &Shared, friend_id: &str) {
    let (api, uuid) = {
        let mut c = shared.lock().await;
        c.guests.remove(friend_id);
        emit_guests(&mut c);
        (c.host.api(), c.db.permissions(friend_id).ok().flatten().and_then(|p| p.2))
    };
    if let (Some(api), Some(uuid)) = (api, uuid) {
        let _ = tokio::task::spawn_blocking(move || {
            let _ = api.set_enabled(&uuid, false);
            // Con un solo ospite chiudere l'app chiude anche la sua sessione
            api.close_app()
        })
        .await;
    }
}

pub fn emit_guests(c: &mut Core) {
    let list: Vec<&ActiveGuest> = c.guests.values().collect();
    c.emit(events::HOST_GUESTS, json!({ "guests": list }));
    // Il monitor degli input gira solo con ospiti che possono usare almeno un dispositivo
    let i = c.host.inputs;
    let wanted = !c.guests.is_empty() && (i.keyboard || i.mouse || i.controller);
    if wanted && c.input_monitor.is_none() {
        c.input_monitor = Some(crate::input_monitor::InputMonitor::start(c.event_sender()));
    } else if !wanted {
        c.input_monitor = None;
    }
}

/// Ricontrolla fasce orarie e scadenze ogni minuto mentre si ospita.
pub async fn run(shared: Shared) {
    loop {
        tokio::time::sleep(Duration::from_secs(60)).await;
        apply_all(&shared).await;
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn weekday_math() {
        // Lunedì 2026-11-16 12:00 UTC
        let ts: i64 = 1_794_830_400;
        let days = ts.div_euclid(86_400);
        assert_eq!((days + 3).rem_euclid(7), 0);
    }
}
