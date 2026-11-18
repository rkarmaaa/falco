# 0006 · Spike: chiavi di sunshine.conf e API REST

- **Stato**: accettato (verificato su Sunshine 2026.914 il 2026-11-18)
- **Sezioni della specifica**: §8.1, §18

## Risultati

- **Istanza dedicata**: Falco avvia `sunshine.exe <cartella-dati>/sunshine/sunshine.conf` come processo proprio, con file isolati tramite `file_apps`, `credentials_file`, `file_state`, `log_path`, `pkey`, `cert`. Non tocca un Sunshine già installato. `system_tray = disabled` nasconde l'icona di Sunshine.
- **Credenziali**: `sunshine.exe <conf> --creds falco <password>`; la password è casuale e sta nel portachiavi (`app.falco.desktop` / `sunshine-web`).
- **Chiavi verificate**: tutte quelle del §8.1 esistono. Correzione: `min_log_level` è **numerico** (0–6), `3` = warning. Valori ammessi verificati: `amd_usage` (`ultralowlatency`…), `amd_rc` (`cbr`, `vbr_latency`, `cqp`), `amd_quality` (`speed`, `balanced`, `quality`), `gamepad` (`x360`, `ds4`, `ds5`, `auto`).
- **Porte**: API web su porta base + 1 (`https://localhost:47990`), certificato autofirmato, autenticazione Basic.
- **Avvio**: la prima volta Sunshine prova gli encoder (circa 15–20 s); il core attende che l'API risponda prima di segnare l'host «pronto».
- **API usate** (`falco-engines::sunshine_api`):
  - `GET /api/pin` → richieste di pairing in attesa `{ pairings: [{ id, name, address }] }`
  - `POST /api/pin` `{ pairing_id, pin, name }` → conferma (nelle versioni recenti serve `pairing_id`)
  - `GET /api/clients/list`, `POST /api/clients/unpair` `{ uuid }`
  - `POST /api/clients/update` `{ uuid, enabled }` → blocca o sblocca un client **senza fork**
  - `POST /api/apps/close` → termina la sessione

## Ancora aperto

- [ ] Sunshine come **servizio di Windows** per catturare UAC e schermata di blocco: va creato dall'installer (M6) con un descrittore che permetta all'utente di avviarlo e fermarlo. In M1 Sunshine gira come processo utente con priorità alta: funziona per giochi e desktop, non per la schermata UAC.
- [ ] Permessi per client (§7) oltre ad abilita/blocca: richiedono il fork (ADR 0011).
- [ ] Con più istanze sulla stessa macchina entrambe si annunciano via mDNS: innocuo, da disattivare nel fork.
