# 0002 · IPC JSON-RPC tra core e UI

- **Stato**: accettato
- **Data**: 2026-11-17
- **Sezioni della specifica**: §2, §11

## Contesto

La UI deve leggere e modificare lo stato senza parlare con Tailscale o i motori, senza porte TCP, e nessun altro processo locale deve poter pilotare il core.

## Decisione

- JSON-RPC 2.0, **un messaggio JSON per riga**, sulla named pipe `falco-core-<utente>` (Windows) o sul socket `core.sock` nella cartella dati locale (macOS), tramite `interprocess`.
- Windows: descrittore SDDL `D:P(A;;GA;;;SY)(A;;GA;;;OW)`, cioè accesso solo a SYSTEM e al proprietario. macOS: permessi `0600`.
- Il trasporto vive in `falco-proto` (feature `transport`), condiviso da core e UI.
- Eventi come notifiche JSON-RPC in broadcast a tutti i client. In aggiunta alla specifica: `config.changed`.
- Metodi previsti ma non ancora implementati rispondono con il codice `-32001` e `data.milestone`.
- La seconda istanza di `falco-core` invia `app.openUi` al core esistente ed esce.
- Contratto completo in [docs/ipc.md](../ipc.md).

## Conseguenze

- La UI Tauri fa da ponte: comando `rpc` verso il core, eventi `core://event`, `core://connected`, `core://disconnected`; si ricollega da sola ogni 2 s.
- Nel browser la UI usa un core simulato con lo stesso contratto (sviluppo e test visivi).
- Da verificare su Windows con un secondo account utente che la pipe rifiuti la connessione.
