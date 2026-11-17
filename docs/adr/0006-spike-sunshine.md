# 0006 · Spike: chiavi di sunshine.conf e API REST

- **Stato**: proposto (da verificare)
- **Data**: 2026-11-17
- **Sezioni della specifica**: §8.1, §18

## Domanda

Quali sono i nomi esatti delle chiavi di `sunshine.conf` e degli endpoint REST nella versione di Sunshine da forkare?

## Stato attuale

`falo-engines::sunshine` genera le chiavi della tabella del §8.1: `encoder`, `port`, `channels`, `upnp`, `fec_percentage`, `origin_web_ui_allowed`, `hevc_mode`, `av1_mode`, `min_log_level`, `amd_usage`, `amd_rc`, `amd_quality`, `amd_preanalysis`, `vt_realtime`, `gamepad`. `hevc_mode` e `av1_mode` sono scritti come `0` (automatico).

## Da verificare

- [ ] Scegliere il tag di Sunshine e creare il fork `falo-sunshine`.
- [ ] Confrontare ogni chiave con la documentazione di configurazione di quel tag; correggere `generate_conf` e i test.
- [ ] Endpoint per il PIN (`POST /api/pin`), elenco e revoca dei client, credenziali (`sunshine --creds`).
- [ ] Avvio con file di configurazione esterno e come servizio su Windows.
- [ ] Porte effettive rispetto a `port` (§9).
