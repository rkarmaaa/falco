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

/// Permessi cambiati per un amico collegato: glieli manda, così le sue icone (tastiera, mouse,
/// controller) si aggiornano subito. Senza risposta non cambia nulla: in Sunshine valgono già.
pub async fn push_permissions(shared: &Shared, friend_id: &str) {
    let (me, perms) = {
        let c = shared.lock().await;
        if !c.guests.contains_key(friend_id) {
            return;
        }
        (c.me.as_ref().map(|m| m.identity.clone()), effective(&c, friend_id))
    };
    let (Some(me), Some(perms)) = (me, perms) else { return };
    let Some(addr) = crate::social::reach(shared, friend_id).await else { return };
    let Ok(body) = serde_json::to_value(&perms) else { return };
    let sent = tokio::task::spawn_blocking(move || {
        crate::social::fpp_call(
            &me,
            "POST",
            &addr,
            "/v1/session/permissions",
            Some(&body),
            None,
            std::time::Duration::from_secs(5),
        )
    })
    .await;
    if !matches!(sent, Ok(Ok((204, _, _)))) {
        tracing::warn!("permessi nuovi non recapitati all'ospite {friend_id}");
    }
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
    // Una sola volta per esecuzione: prima partiva PowerShell (con un lampo di finestra) a ogni
    // controllo dei permessi
    static OFFSET: std::sync::OnceLock<i64> = std::sync::OnceLock::new();
    *OFFSET.get_or_init(read_offset_s)
}

#[cfg(windows)]
fn read_offset_s() -> i64 {
    use std::os::windows::process::CommandExt;
    let out = std::process::Command::new("powershell")
        .args([
            "-NoProfile",
            "-Command",
            "[int][TimeZoneInfo]::Local.GetUtcOffset([DateTime]::Now).TotalMinutes",
        ])
        .creation_flags(0x0800_0000) // CREATE_NO_WINDOW
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
    // Filtro per ospite dentro Sunshine: dispositivi accesi, decide il permesso di ciascuno
    if c.host.per_client {
        return falco_engines::sunshine::Inputs { keyboard: true, mouse: true, controller: true };
    }
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
    // Con il filtro per ospite i permessi valgono subito: niente riavvio, nessuno si disconnette
    if shared.lock().await.host.per_client {
        apply_all(shared).await;
        return;
    }
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
/// Cosa applicare in Sunshine per un amico.
struct Plan {
    friend: String,
    name: String,
    uuid: Option<String>,
    enabled: bool,
    keyboard: bool,
    mouse: bool,
    controller: bool,
}

pub async fn apply_all(shared: &Shared) {
    let (api, plan) = {
        let c = shared.lock().await;
        let Some(api) = c.host.api() else { return };
        let friends = c.db.friends().unwrap_or_default();
        let per_client = c.host.per_client;
        let plan: Vec<Plan> = friends
            .iter()
            .filter_map(|f| {
                let uuid = c.db.permissions(&f.id).ok().flatten()?.2;
                let p = effective(&c, &f.id)?;
                let on = should_enable(&c, &f.id, &p);
                Some(Plan {
                    friend: f.id.clone(),
                    name: f.name.clone(),
                    uuid,
                    enabled: on,
                    keyboard: on && p.keyboard,
                    mouse: on && p.mouse,
                    controller: on && p.gamepads > 0,
                })
            })
            .collect();
        (api, (plan, per_client))
    };
    let (plan, per_client) = plan;
    let learned = tokio::task::spawn_blocking(move || {
        let current = api.clients().unwrap_or_default();
        let owners: Vec<Owner> =
            plan.iter().map(|p| Owner { name: p.name.clone(), uuid: p.uuid.clone() }).collect();
        let mut learned = Vec::new();
        for (p, uuids) in plan.iter().zip(clients_of(&owners, &current)) {
            // Uuid salvato assente o sparito (pairing rifatto): si impara il più recente
            if !uuids.is_empty() && p.uuid.as_ref().is_none_or(|u| !uuids.contains(u)) {
                learned.push((p.friend.clone(), uuids[uuids.len() - 1].clone()));
            }
            for uuid in &uuids {
                if current.iter().any(|c| c.uuid == *uuid && c.enabled != p.enabled) {
                    let _ = api.set_enabled(uuid, p.enabled);
                }
                if per_client
                    && let Err(e) = api.set_inputs(uuid, p.keyboard, p.mouse, p.controller)
                {
                    tracing::warn!("permessi di input per {} non applicati: {e}", p.name);
                }
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

/// Amico visto da Sunshine: nome con cui è stato associato e uuid salvato.
struct Owner {
    name: String,
    uuid: Option<String>,
}

/// Client di Sunshine di ciascun amico, dal più vecchio al più recente: l'uuid salvato più quelli
/// con il suo nome (un pairing rifatto aggiunge un client e non toglie il vecchio). I permessi
/// vanno applicati a tutti: il certificato in uso è uno di questi e non si sa quale. Il nome non
/// basta se due amici si chiamano uguale, e un uuid salvato da un amico non va mai a un altro.
/// Nessun client viene tolto qui: rimuovere un «duplicato» toglieva a volte quello in uso (401).
fn clients_of(
    owners: &[Owner],
    current: &[falco_engines::sunshine_api::PairedClient],
) -> Vec<Vec<String>> {
    owners
        .iter()
        .map(|o| {
            let unique_name = owners.iter().filter(|x| x.name == o.name).count() == 1;
            current
                .iter()
                .filter(|c| {
                    let saved = o.uuid.as_deref() == Some(c.uuid.as_str());
                    let someone_elses =
                        owners.iter().any(|x| x.uuid.as_deref() == Some(c.uuid.as_str()));
                    saved || (unique_name && c.name == o.name && !someone_elses)
                })
                .map(|c| c.uuid.clone())
                .collect()
        })
        .collect()
}

/// Attesa massima di una condivisione in avvio quando qualcuno bussa.
pub const HOST_STARTUP_WAIT_S: u64 = 30;

/// La condivisione è pronta? Se è in avvio (per esempio subito dopo «Invita», quando l'invito
/// arriva prima che Sunshine abbia provato gli encoder) si aspetta invece di rifiutare.
async fn hosting_ready(shared: &Shared) -> bool {
    for _ in 0..HOST_STARTUP_WAIT_S * 2 {
        {
            let c = shared.lock().await;
            if c.host.api().is_some() {
                return true;
            }
            if c.host.state != crate::host::HostState::Starting {
                return false;
            }
        }
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
    false
}

/// Un amico bussa (§7 «Bussare»): controlli dell'host e, se serve, approvazione.
pub async fn knock(shared: &Shared, friend_id: &str) -> KnockAnswer {
    let declined = |r: &str| KnockAnswer::Declined { reason: r.to_owned() };
    if !hosting_ready(shared).await {
        return declined("not-hosting");
    }
    let (perms, name, full, timeout) = {
        let c = shared.lock().await;
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
    use super::{Owner, clients_of};
    use falco_engines::sunshine_api::PairedClient;

    fn sc(uuid: &str, name: &str) -> PairedClient {
        PairedClient { uuid: uuid.into(), name: name.into(), enabled: true }
    }

    fn owner(name: &str, uuid: Option<&str>) -> Owner {
        Owner { name: name.into(), uuid: uuid.map(Into::into) }
    }

    #[test]
    fn permissions_reach_every_client_of_a_repaired_friend() {
        // Pairing rifatto: uuid salvato vecchio, client nuovo con lo stesso nome
        let current = [sc("OLD", "Marco"), sc("NEW", "Marco"), sc("G1", "Gio")];
        let got = clients_of(&[owner("Marco", Some("OLD")), owner("Gio", None)], &current);
        assert_eq!(got, vec![vec!["OLD".to_string(), "NEW".into()], vec!["G1".into()]]);
        // Uuid salvato sparito: restano quelli con il nome
        let got = clients_of(&[owner("Marco", Some("GONE"))], &current);
        assert_eq!(got, vec![vec!["OLD".to_string(), "NEW".into()]]);
    }

    #[test]
    fn same_name_friends_never_share_clients() {
        let current = [sc("A", "Luca"), sc("B", "Luca"), sc("C", "Luca")];
        let got = clients_of(&[owner("Luca", Some("A")), owner("Luca", Some("B"))], &current);
        // Il nome è ambiguo: solo l'uuid salvato, «C» non va a nessuno dei due
        assert_eq!(got, vec![vec!["A".to_string()], vec!["B".into()]]);
        // Uuid salvato da un altro amico (rinominato come lui): mai preso per nome
        let got = clients_of(&[owner("Luca", None), owner("Gio", Some("A"))], &current);
        assert_eq!(got, vec![vec!["B".to_string(), "C".into()], vec!["A".into()]]);
    }

    #[test]
    fn weekday_math() {
        // Lunedì 2026-11-16 12:00 UTC
        let ts: i64 = 1_794_830_400;
        let days = ts.div_euclid(86_400);
        assert_eq!((days + 3).rem_euclid(7), 0);
    }
}
