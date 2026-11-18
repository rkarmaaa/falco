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

- Concetto «volo notturno»: il falco pellegrino è il più veloce degli uccelli e vede da lontano, come uno stream a bassa latenza tra amici.
- Simbolo: testa di falco pellegrino di profilo verso destra, cappuccio Cielo con il baffo, occhio e cera ambra, su quadrato arrotondato grafite.
- Palette: accento **Cielo** `#4CA6FF` al posto della brace; l'ambra `#F5A524` resta solo nell'occhio e nelle illustrazioni, per non confondersi con lo stato «occupato».
- Token SCSS neutri (`$accent-*`, `$on-accent`, `$eye`): un futuro cambio di colore non richiede rinomine.
- Illustrazione degli stati vuoti: falco in planata a tratto (`FalconArt.vue`).

## Da fare

- [ ] Verifica del marchio «Falco» nel settore del software.
- [ ] Nome dell'organizzazione e del repository GitHub.
- [ ] Rifinitura grafica delle schermate in una fase dedicata.
