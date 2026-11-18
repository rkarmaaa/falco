# 0013 · Nome e marchio: Falco

- **Stato**: accettato (richiesta di Marco del 2026-11-18)
- **Sezioni della specifica**: §1, §3, §12

## Decisione

Il progetto e l'app si chiamano **Falco** (prima Falò). La rinomina è completa:

| Ambito | Prima | Ora |
| --- | --- | --- |
| Crate e binari | `falo-*`, `falo-core`, `falo-ui` | `falco-*`, `falco-core`, `falco-ui` |
| Link di invito | `falo://invite/…` | `falco://invite/…` |
| Intestazioni FPP | `X-Falo-Id/Ts/Sig` | `X-Falco-Id/Ts/Sig` (FPP = Falco Presence Protocol) |
| IPC | pipe `falo-core-<utente>` | pipe `falco-core-<utente>` |
| Dati utente | `…\Falo\Falo\` | `…\Falco\Falco\` |
| Portachiavi | `app.falo.desktop` | `app.falco.desktop` |
| Identificatore app | — | `app.falco.desktop` |
| Variabili d'ambiente | `FALO_*` | `FALCO_*` |

Non c'erano utenti né rilasci: nessuna migrazione dei dati, dei link o del portachiavi.

## Marchio

- Logo disegnato da Marco (2026-11-18): testa di rapace di profilo in retino bianco e nero su gradiente indaco `#32409A` → `#111634`. Sostituisce il marchio provvisorio disegnato a mano.
- Accento dell'interfaccia preso dal punto più chiaro dello sfondo del logo: `#32409A`. Essendo scuro si usa per i riempimenti con testo bianco; per testo, bordi e focus sul tema scuro c'è la tinta `$accent-fg` `#8C98EC`.
- Superfici del tema scuro con una sfumatura blu notte coerente con il logo.
- Token SCSS neutri (`$accent-*`, `$accent-fg`, `$on-accent`, `$eye`): un futuro cambio di colore non richiede rinomine.
- Formati: SVG sorgente, PNG da 16 a 1024 px, ICO, ICNS, icone Tauri, favicon (elenco in `DESIGN_SYSTEM.md`).
- Illustrazione degli stati vuoti: falco in planata a tratto (`FalconArt.vue`), da rifare nello stile del logo nella fase grafica.

## Da fare

- [ ] Verifica del marchio «Falco» nel settore del software.
- [ ] Nome dell'organizzazione e del repository GitHub.
- [ ] Rifinitura grafica delle schermate in una fase dedicata.
