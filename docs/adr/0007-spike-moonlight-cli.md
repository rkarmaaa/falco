# 0007 · Spike: CLI di moonlight-qt

- **Stato**: accettato (verificato su moonlight-qt 6.2.0 il 2026-11-18)
- **Sezioni della specifica**: §6, §8.2, §18

## Risultati

- `pair <host> --pin <PIN>` **esiste già**: nessuna patch necessaria per il pairing. Dopo il pairing però Moonlight resta aperto con una finestra: il core verifica l'esito con `list <host>` (codice 0 ed elenco delle app solo se associato) e poi chiude il processo.
- `stream <host> "<app>"` accetta tutti i flag previsti e in più `--audio-config`, `--capture-system-keys`, `--performance-overlay`, `--game-optimization`, `--keep-awake`, `--multi-controller`, `--video-decoder`, `--hdr`, `--yuv444`.
- Gli host con porta non standard si indicano come `indirizzo:porta`.
- **Impostazioni separate**: con un file `portable.dat` **nella cartella di lavoro** Moonlight salva tutto in `Moonlight Game Streaming Project/Moonlight.ini` accanto a sé. Il core avvia sempre i motori con la loro cartella come cartella di lavoro, quindi il Moonlight di Falò non si mescola con uno installato dall'utente.
- **Stato senza fork**: in attesa dello stato JSON del fork, il core legge i log di moonlight-qt: `Starting RTSP handshake` = in collegamento, `Starting input stream` = connesso, uscita del processo = fine sessione.

## Ancora aperto (patch del fork)

- [ ] Stato JSON su stdout con `stats` ogni 2 s (ping, fps, bitrate, percorso).
- [ ] `pair` che esce da solo con un codice di esito.
- [ ] Modalità senza finestra principale e overlay Falò.
