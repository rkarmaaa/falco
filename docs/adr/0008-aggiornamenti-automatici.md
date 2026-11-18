# 0008 · Notifica di aggiornamento e aggiornamento automatico

- **Stato**: accettato e implementato (`apps/core/src/updater.rs`); attivo dalla prima versione firmata
- **Data**: 2026-11-18
- **Sezioni della specifica**: §11, §13, §16 (M6)

## Contesto

Richiesta di Marco: avvisare quando c'è una nuova versione, con un pulsante dedicato, e meglio ancora aggiornare da soli senza installazioni manuali. La specifica prevedeva già l'updater di Tauri con manifest firmato (§11), ma non l'esperienza utente.

## Decisione proposta

- **Controllo nel core**, non nella UI che spesso è chiusa: all'avvio e ogni 6 ore, mai durante lo streaming, leggendo un manifest `latest.json` firmato su GitHub Releases.
- **Libreria**: formato del manifest e firme minisign dell'updater di Tauri (`tauri-plugin-updater`), così manifest e firme si generano con gli strumenti di build di Tauri. Il core usa la stessa verifica senza webview.
- **Modalità** da `general.updates`:
  - `auto` (default): download in background, installazione silenziosa quando non c'è una sessione attiva o alla chiusura, poi riavvio di core e tray.
  - `notify`: notifica di sistema e toast «È disponibile Falco X.Y» con «Aggiorna ora» e «Più tardi».
  - `off`: solo controllo manuale da Impostazioni → Info.
- **Interfaccia**: voce «Aggiorna a X.Y» nel menu del tray, badge d'accento su Impostazioni → Info, evento IPC `update.available`, metodi `update.check` e `update.install`.
- **Pacchetti**: Windows con installer NSIS in modalità silenziosa; macOS sostituendo il bundle `.app` firmato e notarizzato.
- Un aggiornamento comprende core, UI e motori, sempre allineati. Se fallisce resta la versione precedente e la notifica spiega cosa fare.

## Da decidere o verificare

- [ ] Installazione per tutto il sistema (§14) o per utente: la seconda evita una richiesta UAC a ogni aggiornamento.
- [ ] Riavvio del servizio Sunshine durante l'aggiornamento su Windows.
- [ ] Generazione e custodia della chiave di firma degli aggiornamenti (segreto della CI).
- [ ] Serve la firma del codice di M6, altrimenti SmartScreen e Gatekeeper bloccano l'installazione silenziosa.
