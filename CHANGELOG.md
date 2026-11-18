# Changelog

Formato [Keep a Changelog](https://keepachangelog.com/it/1.1.0/), versioni [SemVer](https://semver.org/lang/it/).

## [Non rilasciato]

### Aggiunto

- Monorepo: workspace Cargo (`apps/core`, `apps/ui/src-tauri`, `crates/*`) e UI Tauri 2 + Vue 3.
- `falco-core`: icona nel tray con stato, «Disponibile», «Apri Falco», «Esci»; server JSON-RPC su named pipe o socket Unix; istanza singola.
- `falco-proto`: identità Ed25519, profili firmati in JSON canonico (RFC 8785), ruoli e permessi del §7, link di invito firmati.
- `falco-store`: `config.toml` validato che conserva commenti e chiavi sconosciute; SQLite con migrazioni.
- `falco-net`: provider Tailscale via CLI e primi controlli del Dottore di rete.
- `falco-engines`: generazione di `sunshine.conf` e `apps.json`, argomenti e parametri automatici di Moonlight.
- UI: design system, schermate Amici (stato vuoto), Ospita, Inviti, Impostazioni con Dottore di rete; italiano e inglese; tema scuro e chiaro.
- Documentazione: README, DESIGN_SYSTEM, CONTRIBUTING, SECURITY, TODO, ADR.
