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

## M5 · Rete e diagnostica (fatto)

- [x] Provider dietro una scelta unica: Tailscale (default), **LAN** con mDNS `_falco._tcp` e indirizzo locale preferito, **Diretto** (tutte le interfacce, UPnP, cifratura di Sunshine obbligatoria, IP pubblico negli inviti), **Headscale** (login con `--login-server`)
- [x] Dottore di rete completo: Tailscale, UDP e NAT (`netcheck`), percorso verso ogni amico (`tailscale ping`), porte di Sunshine, banda (`/v1/speedtest`), jitter e perdite (200 pacchetti UDP), Wi-Fi e banda, batteria, risparmio energetico, altre VPN
- [x] Impostazioni avanzate complete del §15 generate dallo schema (`config.schema`), con momento di applicazione; esporta e importa
- [x] Eventi della LocalAPI di Tailscale (`debug watch-ipn`) al posto del solo polling
- [ ] Provider Diretto e LAN da provare su reti vere (**Marco**)

## M6 · Rilascio 1.0 (codice pronto: 1.0.0-rc.1)

- [x] Installer NSIS per tutto il sistema con core, UI e motori; regole firewall limitate a Tailscale; chiusura dei processi prima di aggiornare o disinstallare (`scripts/package.ps1`, `packaging/windows/hooks.nsh`)
- [x] Bundle macOS configurato: `Info.plist` con `falco://`, Bonjour e permessi spiegati, entitlements
- [x] Avvio con il sistema nel tray, schema `falco://` registrato, apertura del link nella scheda «Accetta invito»
- [x] Tailscale installato da Falco con un clic (MSI/PKG ufficiale, firma verificata), ADR 0012 opzione A
- [x] **Notifica di nuova versione** con pulsante (toast, voce nel tray, Impostazioni → Info) e **aggiornamento automatico** silenzioso quando non si gioca; firma minisign verificata (ADR 0008)
- [x] Workflow di rilascio: tag `v*` → pacchetti firmati per l'updater e `latest.json` su GitHub Releases
- [x] Italiano e inglese completi (test di parità delle chiavi), errori in tre parti
- [ ] **Marco**: chiave di firma degli aggiornamenti (`npx tauri signer generate`), da mettere nei segreti del repository e in `updater::PUBLIC_KEY`
- [ ] **Marco**: certificato di firma del codice Windows e Apple Developer Program (notarizzazione)
- [ ] **Marco**: organizzazione GitHub (ora segnaposto `falco-app/falco` nell'URL del manifest)
- [ ] Fork dei motori compilati in CI al posto dei pacchetti ufficiali (`engines/`)
- [ ] Budget del §10 misurati in CI (`falco-core bench`) e accessibilità verificata con screen reader
- [ ] Rifinitura grafica delle schermate (fase dedicata)

## Decisioni da prendere (Marco)

- [ ] **Come arriva Tailscale senza installazione manuale**: installazione silenziosa del client ufficiale oppure `tsnet` dentro Falco ([ADR 0012](docs/adr/0012-nessuna-installazione-manuale.md))
- [ ] Contrasto di `$text-3` e valori del tema chiaro ([DESIGN_SYSTEM.md](DESIGN_SYSTEM.md#questioni-aperte))
- [ ] Verifica del marchio «Falco» nel settore
- [ ] Base dell'host: Sunshine con i permessi di Apollo, oppure Apollo direttamente
- [ ] Budget per la firma del codice su Windows e macOS (serve anche agli aggiornamenti automatici)
- [ ] Supporto ARM64 per Windows in v1 o in v2
- [ ] Nome dell'organizzazione GitHub (ora segnaposto `falco-app/falco` in `Cargo.toml`)
