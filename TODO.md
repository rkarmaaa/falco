# TODO

Stato delle milestone del §16 di [docs/falco-spec.md](docs/falco-spec.md). Una milestone si chiude solo con tutti i criteri spuntati.

## M0 · Fondamenta

- [x] Monorepo come nel §3 (workspace Cargo + app Tauri/Vue)
- [ ] CI su Windows e macOS (arm64 e x86_64): workflow scritto in `.github/workflows/ci.yml`, da verificare al primo push
- [ ] Fork di Sunshine e moonlight-qt compilati in CI, con lo script di sincronizzazione (servono i repository dei fork: vedi [engines/README.md](engines/README.md))
- [ ] Spike chiusi, ognuno con un ADR (chiusi: 0006, 0007; aperti: [0009](docs/adr/0009-spike-tailscale-inviti.md), [0010](docs/adr/0010-spike-webview2.md), [0011](docs/adr/0011-spike-apollo.md))
- [x] Core minimo: icona nel tray, IPC verso la UI, config TOML

Fatto in anticipo, utile alle milestone successive:

- [x] `falco-proto`: identità Ed25519, tag e accento, profili firmati in JSON canonico, ruoli e permessi, link di invito firmati e compressi
- [x] `falco-store`: tabelle SQLite del §5, inviti a uso singolo atomici, config con validazione e conservazione di commenti e chiavi sconosciute
- [x] `falco-net`: lettura di `tailscale status --json`, primi controlli del Dottore di rete
- [x] `falco-engines`: generazione di `sunshine.conf` e `apps.json`, argomenti e parametri automatici di Moonlight, lettura degli eventi JSON
- [x] UI: design system, barra del titolo, barra laterale, stati vuoti, Ospita, Impostazioni con Dottore di rete, italiano e inglese, core simulato nel browser

## M1 · Streaming orchestrato (codice completo, manca la prova su hardware)

- [x] Il core genera `sunshine.conf`, avvia e ferma un'istanza Sunshine dedicata (processo con priorità alta; il **servizio di Windows** arriva con l'installer, M6)
- [x] Il core lancia Moonlight e ne legge lo stato (dai log fino allo stato JSON del fork)
- [x] Pairing con PIN dalla UI provvisoria; con l'host sullo stesso PC il PIN si conferma da solo
- [x] Prova end-to-end in loopback solo via IPC: `node scripts/e2e-loopback.mjs` (HEVC, encoder AMD)
- [x] Comando `falco-core bench [secondi] [stato]` (§10)
- [ ] **Marco**: 1080p60 HEVC da Windows con GPU AMD a MacBook M1 via Tailscale, latenza uguale ai motori originali (telecamera a 240 fps, §17)

## M2 · Identità e amici (fatto)

- [x] Onboarding con nome e avatar (WebP 256×256 ≤ 64 KB), chiave Ed25519 nel portachiavi
- [x] Server di presenza FPP solo sugli IP Tailscale: firma, ±60 s, `tailscale whois`, 20 richieste/min, ETag
- [x] Lista amici (in gioco, online, occupato, offline, in attesa, bloccato) e dettaglio nel pannello destro
- [x] Stato online entro 15 s a finestra aperta (60 s in tray, solo preferiti; fermo in streaming)
- [x] Prova a due istanze: `node scripts/e2e-friends.mjs` (`FALCO_INSTANCE=b` per la seconda)
- [ ] Esportazione e ripristino `.falco-id` (Argon2id + XChaCha20-Poly1305) e collegamento di un secondo dispositivo

## M3 · Inviti (fatto, tranne Tailscale automatico)

- [ ] Installazione automatica di Tailscale nell'onboarding (o `tsnet`, secondo l'ADR 0012)
- [x] OAuth client Tailscale verificato e salvato nel portachiavi; senza OAuth l'amico deve essere già nella tailnet
- [x] Creazione, revoca e scadenza degli inviti; uso singolo atomico; approvazione dell'host per inviti multipli
- [x] Accettazione da link, incolla e appunti; handshake; pairing automatico di Moonlight
- [x] Rimozione e blocco su tre livelli (rifiuto, revoca del pairing, avviso all'amico)
- [ ] Invito Tailscale inverso automatico (passo 9) e revoca della condivisione via API (ADR 0009)
- [ ] **Marco**: un utente senza Tailscale passa dal link a «Siete amici» in meno di 3 minuti (serve l'installazione automatica)

## M4 · Permessi e più ospiti (fatto con Sunshine ufficiale; filtro input dal fork)

- [x] Quattro ruoli, modifica granulare dal pannello dell'amico, scadenza e fasce orarie
- [x] Applicazione lato host con l'API di Sunshine: chi non può entrare resta **bloccato in Sunshine** (non solo nella UI); verifica ogni minuto
- [x] Flusso «bussa» con notifica e Accetta/Rifiuta, timeout, posti massimi; motivo del rifiuto in tre parti
- [x] Ospiti collegati, «Espelli», «Invitalo a giocare», amici in gioco nel menu del tray
- [x] Scorciatoia globale `Ctrl+Shift+F` / `Cmd+Shift+F` verso la vista Ospita
- [x] Prova a due istanze con pairing reale: `node scripts/e2e-friends.mjs` (invito → presenza → entra → bussa → espelli → rifiuto → rimozione)
- [ ] Filtro di controller, tastiera, mouse e qualità **dentro Sunshine** (endpoint `PUT /api/falco/clients/{uuid}/permissions`): serve il fork, ADR 0011
- [ ] Barra rapida sovrapposta al gioco (ora la scorciatoia apre la finestra su Ospita)
- [ ] **Marco**: test con client modificato e tre ospiti contemporanei con controller separati

## M5 · Rete e diagnostica

- [ ] Provider LAN, Diretto e Headscale dietro `NetworkProvider`
- [ ] Dottore di rete con tutti i controlli del §9
- [ ] Impostazioni avanzate complete secondo il §15
- [ ] Eventi della LocalAPI di Tailscale al posto del polling

## M6 · Rilascio 1.0

- [ ] Tutte le schermate del §13 rifinite secondo il design system, in tema scuro e chiaro
- [ ] Italiano e inglese completi, accessibilità verificata
- [ ] Budget del §10 rispettati in CI
- [ ] Installer firmato per Windows, app notarizzata per macOS, aggiornamenti firmati
- [ ] **Notifica di nuova versione** con pulsante «Aggiorna» (toast, voce nel tray, badge su Impostazioni → Info)
- [ ] **Aggiornamento automatico** senza installazioni manuali: download in background e installazione silenziosa quando non c'è una sessione attiva ([ADR 0008](docs/adr/0008-aggiornamenti-automatici.md))
- [ ] Pacchetto unico: Sunshine, Moonlight, driver dei controller e Tailscale arrivano con Falco, senza installazioni manuali ([ADR 0012](docs/adr/0012-nessuna-installazione-manuale.md))
- [ ] Sorgenti GPL-3.0 pubblicati con il rilascio

## Decisioni da prendere (Marco)

- [ ] **Come arriva Tailscale senza installazione manuale**: installazione silenziosa del client ufficiale oppure `tsnet` dentro Falco ([ADR 0012](docs/adr/0012-nessuna-installazione-manuale.md))
- [ ] Contrasto di `$text-3` e valori del tema chiaro ([DESIGN_SYSTEM.md](DESIGN_SYSTEM.md#questioni-aperte))
- [ ] Verifica del marchio «Falco» nel settore
- [ ] Base dell'host: Sunshine con i permessi di Apollo, oppure Apollo direttamente
- [ ] Budget per la firma del codice su Windows e macOS (serve anche agli aggiornamenti automatici)
- [ ] Supporto ARM64 per Windows in v1 o in v2
- [ ] Nome dell'organizzazione GitHub (ora segnaposto `falco-app/falco` in `Cargo.toml`)
