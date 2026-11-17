# TODO

Stato delle milestone del §16 di [docs/falo-spec.md](docs/falo-spec.md). Una milestone si chiude solo con tutti i criteri spuntati.

## M0 · Fondamenta (in corso)

- [x] Monorepo come nel §3 (workspace Cargo + app Tauri/Vue)
- [ ] CI su Windows e macOS (arm64 e x86_64): workflow scritto in `.github/workflows/ci.yml`, da verificare al primo push
- [ ] Fork di Sunshine e moonlight-qt compilati in CI, con lo script di sincronizzazione (servono i repository dei fork: vedi [engines/README.md](engines/README.md))
- [ ] Spike chiusi, ognuno con un ADR (aperti: [0006](docs/adr/0006-spike-sunshine.md), [0007](docs/adr/0007-spike-moonlight-cli.md), [0009](docs/adr/0009-spike-tailscale-inviti.md), [0010](docs/adr/0010-spike-webview2.md), [0011](docs/adr/0011-spike-apollo.md))
- [x] Core minimo: icona nel tray, IPC verso la UI, config TOML

Fatto in anticipo, utile alle milestone successive:

- [x] `falo-proto`: identità Ed25519, tag e accento, profili firmati in JSON canonico, ruoli e permessi, link di invito firmati e compressi
- [x] `falo-store`: tabelle SQLite del §5, inviti a uso singolo atomici, config con validazione e conservazione di commenti e chiavi sconosciute
- [x] `falo-net`: lettura di `tailscale status --json`, primi controlli del Dottore di rete
- [x] `falo-engines`: generazione di `sunshine.conf` e `apps.json`, argomenti e parametri automatici di Moonlight, lettura degli eventi JSON
- [x] UI: design system, barra del titolo, barra laterale, stati vuoti, Ospita, Impostazioni con Dottore di rete, italiano e inglese, core simulato nel browser

## M1 · Streaming orchestrato

- [ ] Il core genera `sunshine.conf`, avvia e ferma Sunshine (servizio su Windows)
- [ ] Il core lancia Moonlight verso un IP Tailscale e legge lo stato JSON
- [ ] Pairing con PIN da una UI provvisoria
- [ ] 1080p60 HEVC da Windows con GPU AMD a MacBook M1, latenza uguale ai motori originali
- [ ] Comando `falo bench` (§10)

## M2 · Identità e amici

- [ ] Onboarding con nome e avatar, chiavi nel portachiavi (`keyring`)
- [ ] Server di presenza FPP con autenticazione e limiti
- [ ] Lista amici con i sei stati del §5, dettaglio nel pannello destro
- [ ] Stato online visibile entro 15 s a finestra aperta

## M3 · Inviti

- [ ] Configurazione guidata dell'OAuth client Tailscale e modalità manuale
- [ ] Creazione, revoca e scadenza degli inviti
- [ ] Accettazione da link, incolla e appunti; handshake; pairing automatico
- [ ] Un utente senza Tailscale passa dal link a «Siete amici» in meno di 3 minuti

## M4 · Permessi e più ospiti

- [ ] Permessi nel fork di Sunshine con l'endpoint del §7
- [ ] Quattro ruoli, modifica granulare, scadenza e fasce orarie
- [ ] Barra rapida dell'host e flusso «bussa»
- [ ] Test con client modificato: nessun input non concesso arriva all'host
- [ ] Tre ospiti contemporanei con controller separati

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
- [ ] Sorgenti GPL-3.0 pubblicati con il rilascio

## Decisioni da prendere (Marco)

- [ ] Contrasto di `$text-3` e valori del tema chiaro ([DESIGN_SYSTEM.md](DESIGN_SYSTEM.md#questioni-aperte))
- [ ] Verifica del marchio «Falò» nel settore
- [ ] Base dell'host: Sunshine con i permessi di Apollo, oppure Apollo direttamente
- [ ] Budget per la firma del codice su Windows e macOS (serve anche agli aggiornamenti automatici)
- [ ] Supporto ARM64 per Windows in v1 o in v2
- [ ] Nome dell'organizzazione GitHub (ora segnaposto `falo-app/falo` in `Cargo.toml`)
