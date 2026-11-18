# Changelog

Formato [Keep a Changelog](https://keepachangelog.com/it/1.1.0/), versioni [SemVer](https://semver.org/lang/it/).

## [1.0.0-rc.2] - 2026-11-18 · Beta

### Aggiunto

- **Falco Hub** (Cloudflare Workers, gratuito): amici per **codice** (`K7QM-2XPA`) o **link**, con richieste d'amicizia da accettare o rifiutare e notifica.
- **Condividi stream** con un clic: avvia la condivisione e crea un link corto; chi lo apre entra subito, senza account.
- **Rete Falco**: Tailscale si installa e si collega da solo con una chiave monouso, senza account per gli utenti. Le chiavi OAuth non servono più (restano facoltative in Rete per chi usa la propria tailnet).
- Badge **BETA**; nuovo selettore della foto profilo.
- Nuova interfaccia: barra in alto compatta, barra delle azioni e dello stream in basso, tema unico scuro «Notte».
- Lo stream resta dentro la finestra di Falco; «Schermo intero» con un pulsantino discreto per uscire.
- Ospiti collegati nella barra con tastiera, mouse e controller che si illuminano quando ricevono input; tasto destro per concedere i singoli dispositivi. Di default si può solo guardare.
- Qualità della condivisione: Bassa latenza, Bilanciato, Connessione lenta, Massima qualità, Personalizzato, e «Trova le impostazioni migliori».
- Ogni errore ha un messaggio chiaro con «Copia» e «Debug» (informazioni dettagliate copiabili).
- Audio condiviso «Solo ciò che è a schermo» (predefinito, Windows): gli ospiti sentono solo le app visibili sullo schermo condiviso, mai Discord e le altre chat vocali.

### Corretto

- Dopo il riavvio l'app poteva chiedere di nuovo nome e foto e poi dire «il profilo esiste già», bloccando la schermata di benvenuto.
- Avvisi in basso a destra: testi altissimi e pulsanti con spazio vuoto.
- Titoli e margini diversi tra Amici e Impostazioni: ora tutte le pagine usano lo stesso layout.
- Chiudere la finestra con la X mostrava l'errore «window|destroy not allowed by ACL».
- Dopo l'installazione la finestra diceva «Non riesco a parlare con Falco»: l'installer avviava il servizio con i privilegi di amministratore e la finestra non poteva collegarsi. Ora lo avvia come utente normale.

### Rimosso

- Tema chiaro e la scelta del tema (`general.theme` viene ignorato).

## [1.0.0-rc.1] - 2026-11-18

### Aggiunto

- **Falco** (prima Falò): nuovo nome, logo e palette con accento indaco `#32409A`.
- Host e client come Parsec: Sunshine dedicato e Moonlight portable pilotati dal core, pairing automatico.
- Identità firmata, amici, presenza peer-to-peer (FPP) solo dentro la tailnet.
- Inviti con un link `falco://`, anteprima dei permessi, rilevamento negli appunti.
- Permessi per amico applicati sull'host: ruoli, bussa, fasce orarie, scadenza, espulsione.
- Provider LAN (mDNS), Diretto e Headscale; Dottore di rete con banda, jitter, porte, NAT e postazione.
- Impostazioni complete del §15 generate dallo schema, esporta e importa.
- Aggiornamenti automatici firmati con notifica; installer Windows con firewall; Tailscale installato in un clic.

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
