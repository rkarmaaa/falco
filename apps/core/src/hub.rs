//! Client di Falco Hub (ADR 0017): codice amico, richieste d'amicizia, link di stream e rete
//! Falco senza account. L'handshake e il pairing restano quelli degli inviti (§6).

use std::collections::HashSet;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use anyhow::{Context, bail};
use falco_net::NetworkProvider;
use falco_proto::hub::{
    self, Accepted, CreateLink, HubLink, Link, NetKey, Register, Requests, SendRequest,
};
use falco_proto::ipc::{TailscaleState, events};
use falco_proto::{Identity, Role, fpp};
use falco_store::config::Provider;
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use tokio::sync::Notify;

use crate::core::{Shared, now};
use crate::invites::{self, CreateParams};

/// Nome già registrato sul Hub in questa esecuzione (si rifà dopo un cambio di nome).
static REGISTERED: AtomicBool = AtomicBool::new(false);
/// Sveglia il ciclo delle richieste (dopo un'azione dell'utente).
static POKE: Notify = Notify::const_new();
/// Un solo tentativo automatico di collegamento alla rete Falco per avvio.
static AUTO_JOINED: AtomicBool = AtomicBool::new(false);

const TIMEOUT: Duration = Duration::from_secs(10);

/// URL del Hub: impostazioni, poi quello della build, poi quello locale nelle build di debug.
pub fn base_url(cfg: &falco_store::Config) -> Option<String> {
    let custom = cfg.network.hub_url.trim();
    let url = if !custom.is_empty() {
        custom
    } else if let Some(built) = option_env!("FALCO_HUB_URL") {
        built
    } else if cfg!(debug_assertions) {
        "http://127.0.0.1:8787"
    } else {
        return None;
    };
    Some(url.trim_end_matches('/').to_owned())
}

pub fn forget_registration() {
    REGISTERED.store(false, Ordering::Relaxed);
}

pub fn poke() {
    POKE.notify_one();
}

/// Richiesta firmata. Bloccante: va eseguita con `spawn_blocking`.
fn call<T: DeserializeOwned>(
    base: &str,
    me: &Identity,
    method: &str,
    path: &str,
    body: Option<&Value>,
) -> anyhow::Result<T> {
    let bytes = body.map(serde_json::to_vec).transpose()?.unwrap_or_default();
    let agent = ureq::Agent::config_builder()
        .timeout_global(Some(TIMEOUT))
        .http_status_as_error(false)
        .build()
        .new_agent();
    let url = format!("{base}{path}");
    let headers = fpp::sign_request(me, method, path, now(), &bytes);
    let mut resp = match method {
        "GET" => headers.iter().fold(agent.get(&url), |r, (k, v)| r.header(*k, v)).call(),
        "DELETE" => headers.iter().fold(agent.delete(&url), |r, (k, v)| r.header(*k, v)).call(),
        _ => headers
            .iter()
            .fold(agent.post(&url).header("content-type", "application/json"), |r, (k, v)| {
                r.header(*k, v)
            })
            .send(&bytes[..]),
    }
    .context("Falco Hub non raggiungibile")?;
    let status = resp.status().as_u16();
    let text = resp.body_mut().read_to_string()?;
    if !(200..300).contains(&status) {
        let msg = serde_json::from_str::<Value>(&text)
            .ok()
            .and_then(|v| v["error"].as_str().map(str::to_owned))
            .unwrap_or_else(|| format!("errore {status} dal Falco Hub"));
        bail!(msg);
    }
    Ok(serde_json::from_str(&text)?)
}

struct Ctx {
    base: String,
    me: Identity,
    name: String,
}

async fn ctx(core: &Shared) -> anyhow::Result<Ctx> {
    let c = core.lock().await;
    let base =
        base_url(c.config.config()).context("Falco Hub non configurato in questa versione")?;
    let me = c.me.as_ref().context("crea prima il tuo profilo")?;
    Ok(Ctx { base, me: me.identity.clone(), name: me.name().to_owned() })
}

async fn request<T: DeserializeOwned + Send + 'static>(
    core: &Shared,
    method: &'static str,
    path: String,
    body: Option<Value>,
) -> anyhow::Result<T> {
    let ctx = ctx(core).await?;
    if !REGISTERED.load(Ordering::Relaxed) {
        let (base, me) = (ctx.base.clone(), ctx.me.clone());
        let reg = serde_json::to_value(Register { name: ctx.name.clone() })?;
        tokio::task::spawn_blocking(move || {
            call::<Value>(&base, &me, "POST", "/v1/me", Some(&reg))
        })
        .await??;
        REGISTERED.store(true, Ordering::Relaxed);
    }
    tokio::task::spawn_blocking(move || call(&ctx.base, &ctx.me, method, &path, body.as_ref()))
        .await?
}

/// Codice amico e link da condividere (calcolati in locale: non serve la rete).
pub async fn info(core: &Shared) -> Value {
    let c = core.lock().await;
    let base = base_url(c.config.config());
    let code = c.me.as_ref().map(|m| hub::code_of(&m.identity.id()));
    json!({
        "configured": base.is_some(),
        "code": code.as_deref().map(hub::display_code),
        "friendLink": base.zip(code).map(|(b, c)| format!("{b}/a/{c}")),
        "network": c.config.config().network.provider == Provider::Falco,
    })
}

pub async fn requests(core: &Shared) -> anyhow::Result<Requests> {
    request(core, "GET", "/v1/requests".into(), None).await
}

/// Richiesta d'amicizia per codice o link: allega un invito a uso singolo (Spettatore, 7 giorni).
pub async fn add_friend(core: &Shared, text: &str) -> anyhow::Result<Value> {
    let Some(HubLink::Add(code)) = hub::parse_link(text) else {
        bail!("codice amico non valido: sono 8 caratteri, come K7QM-2XPA");
    };
    let invite = invites::create(
        core,
        CreateParams {
            role: Role::Spectator,
            validity_hours: hub::REQUEST_VALIDITY_H,
            uses: 1,
            manual_share_url: None,
        },
    )
    .await?
    .link;
    let body = serde_json::to_value(SendRequest { code, invite })?;
    let out = request(core, "POST", "/v1/requests".into(), Some(body)).await;
    poke();
    out
}

/// Accetta una richiesta: il Hub registra l'amicizia e restituisce l'invito da usare.
pub async fn accept(core: &Shared, id: &str) -> anyhow::Result<Accepted> {
    let out = request(core, "POST", format!("/v1/requests/{id}/accept"), None).await;
    poke();
    checked(out?)
}

/// Il Hub è solo un tramite: l'invito deve essere firmato proprio dalla persona indicata.
fn checked(a: Accepted) -> anyhow::Result<Accepted> {
    let p = falco_proto::invite::decode(&a.invite, now())?;
    if p.host.id.to_string() != a.user.id {
        bail!("invito non firmato da {}: richiesta ignorata", a.user.name);
    }
    Ok(a)
}

pub async fn decline(core: &Shared, id: &str) -> anyhow::Result<()> {
    let _: Value = request(core, "POST", format!("/v1/requests/{id}/decline"), None).await?;
    poke();
    Ok(())
}

/// Link corto per guardare (o giocare, secondo il ruolo) la sessione di questo PC.
pub async fn share_stream(core: &Shared, p: CreateParams) -> anyhow::Result<Link> {
    let created = invites::create(core, p).await?;
    let body =
        serde_json::to_value(CreateLink { invite: created.link, expires_at: created.expires_at })?;
    request(core, "POST", "/v1/links".into(), Some(body)).await
}

/// Link di stream aperto: si diventa amici dell'host e si riceve il suo invito.
pub async fn join(core: &Shared, text: &str) -> anyhow::Result<Accepted> {
    let Some(HubLink::Join(code)) = hub::parse_link(text) else {
        bail!("link di stream non valido")
    };
    checked(request(core, "POST", format!("/v1/links/{code}/redeem"), None).await?)
}

/// Amicizia tolta anche dal Hub. Senza Hub non c'è niente da togliere.
pub async fn unfriend(core: &Shared, id: &str) {
    if ctx(core).await.is_err() {
        return;
    }
    if let Err(e) = request::<Value>(core, "DELETE", format!("/v1/friends/{id}"), None).await {
        tracing::warn!("amicizia non tolta dal Hub: {e:#}");
    }
}

/// Collega Tailscale alla rete Falco con una chiave monouso: niente account, niente browser.
pub async fn join_network(core: &Shared) -> anyhow::Result<()> {
    let key: NetKey = request(core, "POST", "/v1/net/key".into(), None).await?;
    let (ts, code) = {
        let c = core.lock().await;
        let code = c.me.as_ref().map(|m| hub::code_of(&m.identity.id())).unwrap_or_default();
        (c.net.clone(), code)
    };
    if !ts.installed() {
        bail!("Tailscale non è installato");
    }
    let hostname = format!("falco-{}", code.to_lowercase());
    let mut args = vec!["login"];
    if !key.login_server.is_empty() {
        args.extend(["--login-server", &key.login_server]);
    }
    args.extend(["--auth-key", &key.auth_key, "--hostname", &hostname, "--accept-dns=false"]);
    if cfg!(windows) {
        // Resta collegato anche senza la sessione utente aperta (servizio di Windows)
        args.push("--unattended");
    }
    ts.run(&args).await.context("collegamento alla rete Falco non riuscito")?;
    let c = core.lock().await;
    c.poke.notify_one();
    c.emit(events::PRESENCE_CHANGED, json!({ "self": { "tailscale": true } }));
    Ok(())
}

/// Rete Falco scelta, profilo pronto e Tailscale installato ma senza login: si collega da solo.
async fn auto_join(core: &Shared) {
    if AUTO_JOINED.load(Ordering::Relaxed) {
        return;
    }
    let (ts, wanted) = {
        let c = core.lock().await;
        let wanted = c.me.is_some() && c.config.config().network.provider == Provider::Falco;
        (c.net.clone(), wanted)
    };
    if !wanted || !ts.installed() {
        return;
    }
    if !ts.status().await.is_ok_and(|n| n.state == TailscaleState::NeedsLogin) {
        return;
    }
    AUTO_JOINED.store(true, Ordering::Relaxed);
    if let Err(e) = join_network(core).await {
        tracing::warn!("rete Falco: {e:#}");
        core.lock().await.errors.push("net.joinFalco", None, &format!("{e:#}"), None);
        AUTO_JOINED.store(false, Ordering::Relaxed);
    }
}

/// Ciclo delle richieste: 20 s a finestra aperta, 2 minuti in tray. Notifica le nuove.
pub async fn run(core: Shared) {
    let mut seen: Option<HashSet<String>> = None;
    let mut last = Requests::default();
    loop {
        auto_join(&core).await;
        if ctx(&core).await.is_ok() {
            match requests(&core).await {
                Ok(r) => {
                    let ids: HashSet<String> = r.incoming.iter().map(|x| x.id.clone()).collect();
                    let c = core.lock().await;
                    if let Some(old) = &seen {
                        for x in r.incoming.iter().filter(|x| !old.contains(&x.id)) {
                            c.notify_user(
                                &format!("{} vuole essere tuo amico", x.user.name),
                                "Apri Falco per accettare o rifiutare.",
                            );
                        }
                    }
                    if r != last {
                        c.emit(events::HUB_REQUESTS, serde_json::to_value(&r).unwrap_or_default());
                        last = r;
                    }
                    seen = Some(ids);
                }
                Err(e) => tracing::debug!("richieste dal Hub: {e:#}"),
            }
        }
        let open = core.lock().await.ui_clients > 0;
        let wait = Duration::from_secs(if open { 20 } else { 120 });
        let _ = tokio::time::timeout(wait, POKE.notified()).await;
    }
}
