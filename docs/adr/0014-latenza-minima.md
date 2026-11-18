# 0014 · Profilo a latenza minima

- **Stato**: accettato (2026-11-18)
- **Sezioni della specifica**: §8, §10, §15, §17

## Contesto

Priorità assoluta di Marco: latenza bassissima. Falco non tocca i pacchetti video, quindi la latenza dipende da come configura i motori e da cosa chiude o lascia aperto durante il gioco.

## Misura

Moonlight stampa a fine sessione le «Global video stats». Il core le legge (`StatsCollector`), le salva nel registro delle sessioni e nell'ultima sessione, le mostra nel Dottore di rete e le restituisce in `session.ended` (`stats`, `latencyMs`).

`latencyMs` = elaborazione sull'host + rete + decodifica + coda + rendering. Esclude la cattura e il tempo del monitor.

Prova ripetibile: `node scripts/latency-bench.mjs [porta] [secondi]` (loopback, 1080p60).

Risultati del 2026-11-18 su Windows 11 con GPU AMD (encoder AMF), loopback:

| Configurazione | Host | Rete | Decodifica | Rendering | Totale | Persi |
| --- | --- | --- | --- | --- | --- | --- |
| HEVC, frame pacing spento | 3,90 ms | 1,00 ms | 0,27 ms | 0,40 ms | **5,58 ms** | 0,24% |
| HEVC, frame pacing acceso | 3,90 | 1,00 | 0,24 | 0,36 | 5,51 | 0,21% |
| H.264, frame pacing spento | 3,90 | 1,00 | 0,25 | 0,37 | 5,53 | 0,21% |
| HEVC, V-Sync acceso | 3,80 | 1,00 | 0,24 | 0,34 | 5,39 | 0,10% |

In finestra le differenze rientrano nel rumore: le statistiche di Moonlight non includono il compositore di Windows, che è proprio dove V-Sync e frame pacing aggiungono ritardo (fino a un fotogramma, 16,7 ms a 60 Hz).

## Decisione

| Dove | Scelta | Perché |
| --- | --- | --- |
| Moonlight | Schermo intero di default (era «senza bordi») | Salta il compositore |
| Moonlight | Frame pacing spento di default (era acceso) | Evita fino a un fotogramma di coda; resta attivabile per chi preferisce la fluidità |
| Moonlight | V-Sync spento, `--game-optimization`, decoder hardware automatico | Nessuna attesa del refresh |
| Moonlight | FPS = frequenza del monitor fino a 120 | Più fotogrammi, meno attesa tra uno e l'altro |
| Sunshine AMD | `ultralowlatency`, CBR, qualità `speed`, pre-analisi spenta | Già nel §8 |
| Sunshine NVIDIA | `nvenc_preset = 1` (P1), due passate spente, latenza prima dei consumi | Preset più rapido |
| Sunshine Intel | `qsv_preset = veryfast` (default `medium`) | Codifica più rapida |
| Sunshine software | `ultrafast` + `zerolatency` | — |
| Processi | Sunshine a priorità alta, Moonlight sopra la normale; UI chiusa durante il gioco | §10 |
| Rete | LAN preferita quando disponibile; Dottore che segnala relay, Wi-Fi a 2,4 GHz, altre VPN | §9 |

## Da verificare (Marco, hardware reale)

- [ ] Misura con telecamera a 240 fps tra due PC (§17): Falco contro Sunshine e Moonlight originali con gli stessi parametri.
- [ ] Schermo intero contro senza bordi su Windows 11 (flip indipendente) e su macOS.
- [ ] HEVC contro AV1 su host e ospite che lo supportano in hardware.
