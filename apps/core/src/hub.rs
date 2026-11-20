//! Client di Falco Hub (ADR 0017): codice amico, richieste d'amicizia, link di stream e rete
//! Falco senza account. L'handshake e il pairing restano quelli degli inviti (§6).

use std::collections::HashSet;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use anyhow::{Context, bail};
use falco_net::NetworkProvider;
use falco_proto::hub::{
    self, Accepted, CreateLink, DirectInvite, HubLink, Link, NetKey, Register, Requests,
    SendRequest, UserRef,
};
use falco_proto::ipc::{TailscaleState, events};
use falco_proto::{Identity, Role, fpp};
use falco_store::config::Provider;
use serde::Deserialize;
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

/// Sito pubblico e repository, per i pulsanti delle Impostazioni.
pub const SITE_URL: &str = "https://falcoapp.pages.dev";
pub const REPO_URL: &str = "https://github.com/rkarmaaa/falco";

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

/// Amici registrati sul Hub, anche quelli mai collegati a questo PC.
pub async fn friends(core: &Shared) -> anyhow::Result<Vec<UserRef>> {
    request(core, "GET", "/v1/friends".into(), None).await
}

/// «Invita» dalla lista: invito a uso singolo (24 h) recapitato dal Hub, anche a chi è offline.
pub async fn invite_friend(core: &Shared, id: &str) -> anyhow::Result<()> {
    let created = invites::create(
        core,
        CreateParams { role: Role::Spectator, validity_hours: 24, uses: 1, manual_share_url: None },
    )
    .await?;
    let body = serde_json::to_value(DirectInvite {
        to: id.to_owned(),
        invite: created.link,
        expires_at: created.expires_at,
    })?;
    let _: Value = request(core, "POST", "/v1/invites".into(), Some(body)).await?;
    poke_friend(core, id).await;
    Ok(())
}

/// Avvisa l'amico, se raggiungibile, di leggere subito il Hub: altrimenti l'invito arriva al suo
/// prossimo giro (fino a 2 minuti con Falco nel tray). Senza risposta non cambia nulla.
async fn poke_friend(core: &Shared, id: &str) {
    let Some(addr) = crate::social::reach(core, id).await else { return };
    let Some(me) = core.lock().await.me.as_ref().map(|m| m.identity.clone()) else { return };
    let _ = tokio::task::spawn_blocking(move || {
        crate::social::fpp_call(
            &me,
            "POST",
            &addr,
            "/v1/hub/poke",
            Some(&json!({})),
            None,
            Duration::from_secs(3),
        )
    })
    .await;
}

/// Invito ricevuto: il Hub lo consegna una volta sola.
pub async fn take_invite(core: &Shared, id: &str) -> anyhow::Result<Accepted> {
    let out = request(core, "POST", format!("/v1/invites/{id}/take"), None).await;
    poke();
    checked(out?)
}

pub async fn decline_invite(core: &Shared, id: &str) -> anyhow::Result<()> {
    let _: Value = request(core, "POST", format!("/v1/invites/{id}/decline"), None).await?;
    poke();
    Ok(())
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
    // Già sulla rete Falco: un nuovo login creerebbe un dispositivo doppione nella tailnet
    let ts = core.lock().await.net.clone();
    if on_falco_network(&ts).await {
        return Ok(());
    }
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
    // Profilo Falco di questo PC già presente (si era passati a un altro account): si torna lì.
    // Un login nuovo creerebbe un dispositivo nuovo che gli amici non conoscono (ADR 0018)
    if switch_to_profile(&ts, &hostname).await {
        let c = core.lock().await;
        c.poke.notify_one();
        c.emit(events::PRESENCE_CHANGED, json!({ "self": { "tailscale": true } }));
        return Ok(());
    }
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

#[derive(Deserialize)]
struct TsProfile {
    id: String,
    account: String,
    #[serde(default)]
    selected: bool,
}

/// Profilo Tailscale salvato per questo dispositivo Falco (`falco-<codice>` o `falco-<codice>-N`,
/// seguito dal dominio MagicDNS), il più recente, se non è già quello attivo.
fn falco_profile<'a>(profiles: &'a [TsProfile], hostname: &str) -> Option<&'a TsProfile> {
    let ours = |p: &&TsProfile| {
        p.account.strip_prefix(hostname).is_some_and(|rest| {
            let rest = rest.split('.').next().unwrap_or_default();
            rest.is_empty() || rest.strip_prefix('-').is_some_and(|n| n.parse::<u32>().is_ok())
        })
    };
    if profiles.iter().filter(ours).any(|p| p.selected) {
        return None;
    }
    profiles.iter().rev().find(ours)
}

/// Passa al profilo Falco già salvato; true se dopo il cambio si è sulla rete Falco.
async fn switch_to_profile(ts: &falco_net::tailscale::Tailscale, hostname: &str) -> bool {
    let Ok(out) = ts.run(&["switch", "--list", "--json"]).await else { return false };
    let profiles: Vec<TsProfile> = serde_json::from_slice(&out).unwrap_or_default();
    let Some(p) = falco_profile(&profiles, hostname) else { return false };
    if let Err(e) = ts.run(&["switch", &p.id]).await {
        tracing::warn!("profilo Tailscale {} non attivato: {e:#}", p.account);
        return false;
    }
    for _ in 0..10 {
        if on_falco_network(ts).await {
            tracing::warn!("rete Falco: tornato al profilo Tailscale {}", p.account);
            return true;
        }
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    }
    false
}

/// Questo PC è collegato alla tailnet di Falco (il suo nodo ha il tag `tag:falco`)?
async fn on_falco_network(ts: &falco_net::tailscale::Tailscale) -> bool {
    let Ok(out) = ts.run(&["status", "--json"]).await else { return false };
    serde_json::from_slice::<Value>(&out).is_ok_and(|v| {
        v["BackendState"] == "Running"
            && v["Self"]["Tags"].as_array().is_some_and(|t| t.iter().any(|x| x == "tag:falco"))
    })
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
    let mut seen_invites: Option<HashSet<String>> = None;
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
                    let invite_ids: HashSet<String> =
                        r.invites.iter().map(|x| x.id.clone()).collect();
                    if let Some(old) = &seen_invites {
                        for x in r.invites.iter().filter(|x| !old.contains(&x.id)) {
                            c.notify_user(
                                &format!("{} ti invita a guardare", x.user.name),
                                "Apri Falco ed entra con un clic.",
                            );
                        }
                    }
                    seen_invites = Some(invite_ids);
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

#[cfg(test)]
mod tests {
    use super::{TsProfile, falco_profile};

    fn p(id: &str, account: &str, selected: bool) -> TsProfile {
        TsProfile { id: id.into(), account: account.into(), selected }
    }

    #[test]
    fn picks_the_newest_saved_profile_of_this_device() {
        let list = [
            p("6100", "someone@gmail.com", true),
            p("5dc9", "falco-bd5b4qx4.tail837df5.ts.net", false),
            p("2da0", "falco-bd5b4qx4-1.tail837df5.ts.net", false),
            p("9999", "falco-bd5b4qx40.tail837df5.ts.net", false),
            p("8888", "falco-zzzzzzzz.tail837df5.ts.net", false),
        ];
        assert_eq!(falco_profile(&list, "falco-bd5b4qx4").map(|x| x.id.as_str()), Some("2da0"));
    }

    #[test]
    fn nothing_to_do_when_already_on_it_or_none_saved() {
        let on = [p("5dc9", "falco-bd5b4qx4.tail837df5.ts.net", true)];
        assert!(falco_profile(&on, "falco-bd5b4qx4").is_none());
        let other = [p("6100", "someone@gmail.com", true)];
        assert!(falco_profile(&other, "falco-bd5b4qx4").is_none());
    }
}
