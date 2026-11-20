//! Inviti (§6): creazione lato host, accettazione lato ospite, pairing automatico.

use std::time::Duration;

use anyhow::{Context, bail};
use falco_net::NetworkProvider;
use falco_net::tailscale_api::TailscaleApi;
use falco_proto::fpp::{DEFAULT_PRESENCE_PORT, HandshakeRequest, HandshakeResponse};
use falco_proto::invite::{self, InviteHost, InvitePayload, InviteTailscale};
use falco_proto::ipc::events;
use falco_proto::{Permissions, Role};
use falco_store::db::{DeviceRow, FriendRow, FriendState};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::sync::oneshot;

use crate::core::{Shared, now};
use crate::social;

const OAUTH_ID: &str = "tailscale-oauth-id";
const OAUTH_SECRET: &str = "tailscale-oauth-secret";

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateParams {
    pub role: Role,
    /// 1, 24 o 168 ore.
    pub validity_hours: u32,
    pub uses: u8,
    /// Link di condivisione incollato a mano (modalità senza OAuth).
    pub manual_share_url: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Created {
    pub link: String,
    pub expires_at: i64,
    pub tailscale_shared: bool,
}

pub fn oauth_configured() -> bool {
    matches!(crate::secrets::get(OAUTH_ID), Ok(Some(_)))
}

pub fn set_oauth(client_id: &str, secret: &str) -> anyhow::Result<()> {
    // Verifica subito le credenziali prima di salvarle
    TailscaleApi::login(client_id, secret).context("credenziali OAuth non valide")?;
    crate::secrets::set(OAUTH_ID, client_id)?;
    crate::secrets::set(OAUTH_SECRET, secret)
}

fn api() -> anyhow::Result<Option<TailscaleApi>> {
    let (Some(id), Some(secret)) =
        (crate::secrets::get(OAUTH_ID)?, crate::secrets::get(OAUTH_SECRET)?)
    else {
        return Ok(None);
    };
    Ok(Some(TailscaleApi::login(&id, &secret)?))
}

/// Crea l'invito: token casuale (solo l'hash resta), invito Tailscale, payload firmato.
pub async fn create(core: &Shared, p: CreateParams) -> anyhow::Result<Created> {
    let uses = p.uses.clamp(1, 10);
    let validity = match p.validity_hours {
        1 | 24 | 168 => i64::from(p.validity_hours),
        _ => 24,
    };
    let ts = core.lock().await.net.clone();
    let net = ts.status().await?;
    let node = net.self_node_id.clone().context("Tailscale non è collegato")?;
    let ip = match crate::netmodes::public_ip(core).await {
        Some(public) => public,
        None => net
            .self_ips
            .iter()
            .find(|ip| ip.is_ipv4())
            .or(net.self_ips.first())
            .context("nessun indirizzo Tailscale")?
            .to_string(),
    };

    // Invito di condivisione: automatico con OAuth, altrimenti quello incollato
    let node2 = node.clone();
    let ts_invite = tokio::task::spawn_blocking(move || -> anyhow::Result<_> {
        Ok(match api()? {
            Some(a) => Some(a.create_device_invite(&node2, uses > 1)?),
            None => None,
        })
    })
    .await??;

    let (token, hash) = invite::new_token();
    let created = now();
    let exp = created + validity * 3600;
    let c = core.lock().await;
    let me = c.me.as_ref().context("crea prima il tuo profilo")?;
    c.db.insert_invite(&hash, &role_name(p.role), created, exp, uses)?;
    if let Some(i) = &ts_invite {
        c.db.set_invite_ts_id(&hash, &i.id)?;
    }
    let port = c.config.config().network.presence_port;
    let invite_url = ts_invite.map(|i| i.url).or(p.manual_share_url).unwrap_or_default();
    let payload = InvitePayload {
        v: 1,
        host: InviteHost {
            id: me.identity.id(),
            name: me.name().to_owned(),
            tag: me.identity.id().tag(),
        },
        ts: InviteTailscale {
            invite_url: invite_url.clone(),
            node: net_host_name(),
            ip,
            port: (port != DEFAULT_PRESENCE_PORT).then_some(port),
        },
        token,
        role: p.role,
        exp,
    };
    let link = invite::encode(payload, &me.identity)?;
    Ok(Created { link, expires_at: exp, tailscale_shared: !invite_url.is_empty() })
}

fn net_host_name() -> String {
    std::env::var("COMPUTERNAME").unwrap_or_else(|_| "falco".into()).to_lowercase()
}

pub fn role_name(r: Role) -> String {
    serde_json::to_value(r).ok().and_then(|v| v.as_str().map(str::to_owned)).unwrap_or_default()
}

/// Revoca: token invalidato subito, invito Tailscale cancellato se possibile.
pub async fn revoke(core: &Shared, token_hash: &str) -> anyhow::Result<()> {
    let ts_id = core.lock().await.db.revoke_invite(token_hash)?;
    if let Some(id) = ts_id {
        tokio::task::spawn_blocking(move || -> anyhow::Result<()> {
            if let Some(a) = api()? {
                a.delete_device_invite(&id)?;
            }
            Ok(())
        })
        .await??;
    }
    Ok(())
}

/// Anteprima per la scheda «Accetta invito»: host, ruolo e permessi a parole.
pub fn preview(link: &str) -> anyhow::Result<serde_json::Value> {
    let p = invite::decode(link, now())?;
    Ok(json!({
        "host": { "id": p.host.id.to_string(), "name": p.host.name, "tag": p.host.tag, "accent": p.host.id.accent() },
        "role": role_name(p.role),
        "permissions": Permissions::resolve(p.role, &Default::default()),
        "expiresAt": p.exp,
        "needsTailscaleShare": !p.ts.invite_url.is_empty(),
    }))
}

/// Richiesta di ingresso da approvare (inviti a più usi).
pub async fn ask_host(core: &Shared, name: &str, tag: &str) -> bool {
    let (tx, rx) = oneshot::channel();
    let id = invite::new_token().1[..12].to_owned();
    {
        let mut c = core.lock().await;
        c.approvals.insert(id.clone(), tx);
        c.emit(events::INVITE_REQUEST, json!({ "requestId": id, "name": name, "tag": tag }));
        c.notify_user(&format!("{name} vuole unirsi"), "Apri Falco per accettare o rifiutare.");
    }
    let ok = matches!(tokio::time::timeout(Duration::from_secs(60), rx).await, Ok(Ok(true)));
    core.lock().await.approvals.remove(&id);
    ok
}

/// Associazioni in corso lato host: finché ce n'è una, Sunshine non si riavvia (si perderebbe
/// la richiesta di Moonlight in attesa del PIN).
static PAIRING: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

pub fn pairing_active() -> bool {
    PAIRING.load(std::sync::atomic::Ordering::Relaxed) > 0
}

struct PairingGuard;

impl PairingGuard {
    fn new() -> Self {
        PAIRING.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        Self
    }
}

impl Drop for PairingGuard {
    fn drop(&mut self) {
        PAIRING.fetch_sub(1, std::sync::atomic::Ordering::Relaxed);
    }
}

/// Lato host: Sunshine attivo e conferma del PIN appena arriva la richiesta di Moonlight.
/// Il client creato da questo pairing: Moonlight finisce lo scambio dopo il PIN (alcune richieste,
/// circa un secondo in rete), quindi si aspetta fino a 15 s un uuid nuovo con quel nome. Prendere
/// il client vecchio con lo stesso nome faceva poi rimuovere quello nuovo come «duplicato» (401).
fn wait_new_client(
    api: &falco_engines::sunshine_api::SunshineApi,
    name: &str,
    known: &[String],
) -> Option<String> {
    for _ in 0..30 {
        std::thread::sleep(Duration::from_millis(500));
        if let Some(uuid) = new_client(&api.clients().unwrap_or_default(), name, known) {
            return Some(uuid);
        }
    }
    None
}

fn new_client(
    clients: &[falco_engines::sunshine_api::PairedClient],
    name: &str,
    known: &[String],
) -> Option<String> {
    clients
        .iter()
        .rev()
        .find(|c| c.name == name && !known.contains(&c.uuid))
        .map(|c| c.uuid.clone())
}

pub async fn host_pairing(
    core: Shared,
    friend_id: String,
    name: String,
    pin: String,
) -> anyhow::Result<()> {
    let was_available = core.lock().await.host_available;
    if core.lock().await.host.api().is_none() {
        crate::core::set_available(core.clone(), true).await;
    }
    let api = core.lock().await.host.api().context("motore host non avviato")?;
    let guard = PairingGuard::new();
    tokio::spawn(async move {
        let _guard = guard;
        let (n, p) = (name.clone(), pin.clone());
        let api2 = api.clone();
        // Riprova ogni 500 ms per 2 minuti, quanto aspetta l'ospite (§6 passo 8). Gli errori
        // finiscono nel log (livello warn): sono la prima cosa da guardare nel report di debug
        let uuid = tokio::task::spawn_blocking(move || {
            let mut last_err = String::new();
            for _ in 0..240 {
                match api2.pending_pairings() {
                    Ok(list) => match list.first() {
                        Some(req) => {
                            // Client già associati prima di questo PIN: con un amico che rifà il
                            // pairing ce n'è uno vecchio con lo stesso nome, da non confondere
                            let known: Vec<String> = api2
                                .clients()
                                .map(|cs| cs.into_iter().map(|c| c.uuid).collect())
                                .unwrap_or_default();
                            match api2.submit_pin(&req.id, &p, &n) {
                                Ok(()) => {
                                    let uuid = wait_new_client(&api2, &n, &known);
                                    if uuid.is_none() {
                                        tracing::warn!(
                                            "pairing {n}: PIN accettato ma client non trovato"
                                        );
                                    }
                                    return uuid;
                                }
                                Err(e) => last_err = format!("PIN rifiutato: {e}"),
                            }
                        }
                        None => last_err = "nessuna richiesta di Moonlight arrivata".into(),
                    },
                    Err(e) => last_err = format!("API di Sunshine: {e}"),
                }
                std::thread::sleep(Duration::from_millis(500));
            }
            tracing::warn!("pairing {n} non riuscito: {last_err}");
            None
        })
        .await
        .ok()
        .flatten();
        tracing::info!(
            "pairing host per {name}: {}",
            if uuid.is_some() { "ok" } else { "fallito" }
        );
        let c = core.lock().await;
        if let Some(uuid) = &uuid {
            let _ = c.db.set_client_cert(&friend_id, uuid);
        }
        c.emit(
            events::INVITE_PROGRESS,
            json!({ "side": "host", "step": "paired", "ok": uuid.is_some(), "name": name }),
        );
        drop(c);
        // L'ospite verifica il pairing collegandosi: un minuto prima di spegnere
        tokio::time::sleep(Duration::from_secs(60)).await;
        // Si spegne solo se nel frattempo l'utente non l'ha reso disponibile
        let keep = {
            let c = core.lock().await;
            c.user_available || !c.guests.is_empty()
        };
        if !was_available && !keep {
            crate::core::set_available(core.clone(), false).await;
        }
    });
    Ok(())
}

fn progress(core: &crate::core::Core, step: &str, ok: Option<bool>) {
    core.emit(events::INVITE_PROGRESS, json!({ "side": "guest", "step": step, "ok": ok }));
}

/// Lato ospite: Tailscale → condivisione → handshake → pairing (§6 «Accettazione»).
/// Inviti in accettazione: lo stesso link aperto due volte non parte due volte.
static ACCEPTING: std::sync::Mutex<Vec<String>> = std::sync::Mutex::new(Vec::new());

/// Segna l'invito come in corso; `false` se lo era già.
pub fn begin_accept(link: &str) -> bool {
    let mut a = ACCEPTING.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    if a.iter().any(|l| l == link) {
        return false;
    }
    a.push(link.to_owned());
    true
}

fn end_accept(link: &str) {
    ACCEPTING.lock().unwrap_or_else(std::sync::PoisonError::into_inner).retain(|l| l != link);
}

pub async fn accept(core: Shared, link: String) -> anyhow::Result<()> {
    let out = accept_inner(core, &link).await;
    end_accept(&link);
    out
}

async fn accept_inner(core: Shared, link: &str) -> anyhow::Result<()> {
    let p = invite::decode(link, now())?;
    let ts = core.lock().await.net.clone();

    // 1. Tailscale installato e collegato
    progress(&*core.lock().await, "tailscale", None);
    let net = ts.status().await?;
    if net.state != falco_proto::ipc::TailscaleState::Running {
        progress(&*core.lock().await, "tailscale", Some(false));
        bail!("Tailscale non è collegato");
    }
    progress(&*core.lock().await, "tailscale", Some(true));

    // 2. Condivisione: il browser apre l'invito, poi si attende che l'host sia visibile
    progress(&*core.lock().await, "share", None);
    let host_ip: std::net::IpAddr = p.ts.ip.parse().context("indirizzo dell'host non valido")?;
    let visible = |n: &falco_net::NetStatus| {
        n.self_ips.contains(&host_ip) || n.peers.iter().any(|peer| peer.ips.contains(&host_ip))
    };
    if !visible(&net) {
        if !p.ts.invite_url.is_empty() {
            let _ = opener::open_browser(&p.ts.invite_url);
        }
        // Rete Falco: l'host è visibile in pochi secondi, oppure non è proprio nella rete
        let falco = core.lock().await.config.config().network.provider
            == falco_store::config::Provider::Falco;
        let mut found = false;
        for _ in 0..if falco { 30 } else { 150 } {
            tokio::time::sleep(Duration::from_secs(2)).await;
            if ts.status().await.is_ok_and(|n| visible(&n)) {
                found = true;
                break;
            }
        }
        if !found {
            progress(&*core.lock().await, "share", Some(false));
            if falco {
                bail!(
                    "il PC di {} non è sulla rete Falco: chiedigli di aprire Impostazioni → Rete → «Collega alla rete Falco» e di mandarti un link nuovo",
                    p.host.name
                );
            }
            bail!("la condivisione su Tailscale non è stata accettata");
        }
    }
    progress(&*core.lock().await, "share", Some(true));
    // ID stabile del nodo dell'host, per il doppio vincolo quando sarà lui a chiamarci
    let net = ts.status().await?;
    let host_node = if net.self_ips.contains(&host_ip) {
        net.self_node_id.clone()
    } else {
        net.peers.iter().find(|x| x.ips.contains(&host_ip)).map(|x| x.node_id.clone())
    }
    .context("nodo Tailscale dell'host non trovato")?;

    // 3. Handshake con l'host
    progress(&*core.lock().await, "handshake", None);
    let (me, req) = {
        let c = core.lock().await;
        let me = c.me.as_ref().context("crea prima il tuo profilo")?;
        let req = HandshakeRequest {
            token: p.token.clone(),
            profile: me.signed(c.status(), social::my_devices(&net))?,
            node_id: net.self_node_id.clone().context("nodo Tailscale sconosciuto")?,
            presence_port: c.config.config().network.presence_port,
            reverse_invite_url: None,
        };
        (me.identity.clone(), req)
    };
    let address = social::join_address(&p.ts.ip, p.ts.port.unwrap_or(DEFAULT_PRESENCE_PORT));
    let (a2, body) = (address.clone(), serde_json::to_value(&req)?);
    let (status, _, bytes) = tokio::task::spawn_blocking(move || {
        social::fpp_call(
            &me,
            "POST",
            &a2,
            "/v1/handshake",
            Some(&body),
            None,
            Duration::from_secs(120),
        )
    })
    .await??;
    if status != 200 {
        progress(&*core.lock().await, "handshake", Some(false));
        let msg = serde_json::from_slice::<serde_json::Value>(&bytes)
            .ok()
            .and_then(|v| v["error"].as_str().map(str::to_owned))
            .unwrap_or_else(|| format!("errore {status}"));
        bail!(msg);
    }
    let resp: HandshakeResponse = serde_json::from_slice(&bytes)?;
    let host = resp.profile.verify()?.clone();
    if host.id != p.host.id {
        bail!("l'host ha risposto con un'identità diversa da quella dell'invito");
    }
    {
        let c = core.lock().await;
        let hid = host.id.to_string();
        c.db.upsert_friend(&FriendRow {
            id: hid.clone(),
            tag: host.tag.clone(),
            name: host.name.clone(),
            avatar_hash: host.avatar.clone(),
            accent: host.accent.clone(),
            state: FriendState::Friend,
            favorite: false,
            added_at: now(),
            last_seen: Some(now()),
        })?;
        c.db.set_friend_profile(&hid, &serde_json::to_string(&resp.profile)?)?;
        c.db.upsert_device(&DeviceRow {
            friend_id: hid,
            ts_node_id: host_node.clone(),
            ts_ip: address.clone(),
            hostname: p.ts.node.clone(),
            os: String::new(),
            can_host: true,
            shared_in: true,
            shared_out: true,
        })?;
        progress(&c, "handshake", Some(true));
        social::emit_friends(&c);
    }

    // 4. Pairing di Moonlight con il PIN dell'host
    progress(&*core.lock().await, "pairing", None);
    let moon = crate::guest::moonlight_address(&p.ts.ip, resp.sunshine_port);
    let ok = crate::guest::pair(core.clone(), moon, resp.pin).await.is_ok();
    let c = core.lock().await;
    progress(&c, "pairing", Some(ok));
    c.poke.notify_one();
    if !ok {
        bail!(
            "{} non ha confermato l'associazione dei dispositivi: riprova con un invito nuovo",
            p.host.name
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::new_client;
    use falco_engines::sunshine_api::PairedClient;

    fn client(uuid: &str, name: &str) -> PairedClient {
        PairedClient { uuid: uuid.into(), name: name.into(), enabled: true }
    }

    #[test]
    fn repairing_picks_the_new_client_not_the_old_one() {
        let known = vec!["OLD".to_string()];
        // Moonlight non ha ancora finito: c'è solo il vecchio client con lo stesso nome
        assert_eq!(new_client(&[client("OLD", "Marco")], "Marco", &known), None);
        // Finito: il nuovo, anche se l'API lo elenca prima del vecchio
        let both = [client("NEW", "Marco"), client("OLD", "Marco")];
        assert_eq!(new_client(&both, "Marco", &known).as_deref(), Some("NEW"));
        // Un altro amico che si associa intanto non conta
        assert_eq!(new_client(&[client("X", "Gio")], "Marco", &[]), None);
    }
}
