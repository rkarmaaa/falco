# 0007 · Spike: CLI di moonlight-qt

- **Stato**: proposto (da verificare)
- **Data**: 2026-11-17
- **Sezioni della specifica**: §6, §8.2, §18

## Stato attuale

`falo-engines::moonlight` produce `stream <host> <app> --resolution WxH --fps N --bitrate K --video-codec HEVC --display-mode borderless --[no-]vsync --[no-]frame-pacing --quit-after` e `pair <host> --pin <PIN>`, e legge eventi JSON (`connecting`, `connected`, `stats`, `ended`) da stdout.

## Da verificare

- [ ] `moonlight stream --help` e `moonlight pair --help` sulla versione scelta.
- [ ] Presenza di `--pin`; altrimenti patch nel fork (branch `falo/pair-pin`).
- [ ] Patch per lo stato JSON su stdout e per la cartella di configurazione separata.
- [ ] Modalità senza finestra principale.
