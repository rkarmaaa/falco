//! Inviti (§6): creazione lato host, accettazione lato ospite, pairing automatico.

use std::time::Duration;

use anyhow::{Context, bail};
use falo_net::NetworkProvider;
use falo_net::tailscale_api::TailscaleApi;
use falo_proto::fpp::{DEFAULT_PRESENCE_PORT, HandshakeRequest, HandshakeResponse};
use falo_proto::invite::{self, InviteHost, InvitePayload, InviteTailscale};
use falo_proto::ipc::events;
use falo_proto::{Permissions, Role};
use falo_store::db::{DeviceRow, FriendRow, FriendState};
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
    let ip = net
        .self_ips
        .iter()
        .find(|ip| ip.is_ipv4())
        .or(net.self_ips.first())
        .context("nessun indirizzo Tailscale")?
        .to_string();

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
    std::env::var("COMPUTERNAME").unwrap_or_else(|_| "falo".into()).to_lowercase()
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
        c.notify_user(&format!("{name} vuole unirsi"), "Apri Falò per accettare o rifiutare.");
    }
    let ok = matches!(tokio::time::timeout(Duration::from_secs(60), rx).await, Ok(Ok(true)));
    core.lock().await.approvals.remove(&id);
    ok
}

/// Lato host: Sunshine attivo e conferma del PIN appena arriva la richiesta di Moonlight.
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
    tokio::spawn(async move {
        let (n, p) = (name.clone(), pin.clone());
        let api2 = api.clone();
        // Riprova ogni 500 ms per 30 s (§6 passo 8)
        let uuid = tokio::task::spawn_blocking(move || {
            for _ in 0..60 {
                if let Ok(list) = api2.pending_pairings()
                    && let Some(req) = list.first()
                    && api2.submit_pin(&req.id, &p, &n).is_ok()
                {
                    std::thread::sleep(Duration::from_millis(500));
                    return api2
                        .clients()
                        .ok()
                        .and_then(|cs| cs.into_iter().rev().find(|c| c.name == n))
                        .map(|c| c.uuid);
                }
                std::thread::sleep(Duration::from_millis(500));
            }
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
pub async fn accept(core: Shared, link: String) -> anyhow::Result<()> {
    let p = invite::decode(&link, now())?;
    let ts = core.lock().await.net.clone();

    // 1. Tailscale installato e collegato
    progress(&*core.lock().await, "tailscale", None);
    let net = ts.status().await?;
    if net.state != falo_proto::ipc::TailscaleState::Running {
        progress(&*core.lock().await, "tailscale", Some(false));
        bail!("Tailscale non è collegato");
    }
    progress(&*core.lock().await, "tailscale", Some(true));

    // 2. Condivisione: il browser apre l'invito, poi si attende che l'host sia visibile
    progress(&*core.lock().await, "share", None);
    let host_ip: std::net::IpAddr = p.ts.ip.parse().context("indirizzo dell'host non valido")?;
    let visible = |n: &falo_net::NetStatus| {
        n.self_ips.contains(&host_ip) || n.peers.iter().any(|peer| peer.ips.contains(&host_ip))
    };
    if !visible(&net) {
        if !p.ts.invite_url.is_empty() {
            let _ = opener::open_browser(&p.ts.invite_url);
        }
        let mut found = false;
        for _ in 0..150 {
            tokio::time::sleep(Duration::from_secs(2)).await;
            if ts.status().await.is_ok_and(|n| visible(&n)) {
                found = true;
                break;
            }
        }
        if !found {
            progress(&*core.lock().await, "share", Some(false));
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
        bail!("pairing con l'host non riuscito");
    }
    Ok(())
}
