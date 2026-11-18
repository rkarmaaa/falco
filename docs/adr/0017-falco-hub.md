# 0017 · Falco Hub: codici amico, richieste, link di stream e rete senza account

- **Stato**: accettato (codice pronto; il Worker va pubblicato dall'account Cloudflare di Marco)
- **Data**: 2026-11-18
- **Sezioni della specifica**: §1, §4, §6, §9, §19
- **Sostituisce in parte**: 0009 (l'OAuth per utente diventa facoltativo), 0012 (resta l'opzione A, senza login)

## Contesto

Prove di Marco: aggiungere un amico e invitarlo a guardare richiede troppi passi (link firmati lunghi,
chiavi OAuth di Tailscale, condivisione del nodo, login a Tailscale). Requisiti nuovi:

1. Un servizio HTTPS per aggiungere amici e invitare alle sessioni.
2. Amici per **codice** (quello già mostrato, `#K7QM-2XPA`) o per **link**, con una lista di
   richieste da accettare o rifiutare.
3. «Aggiungi amico» e «Condividi stream» separati; lo stream si condivide con un link corto.
4. Nessuna installazione o account Tailscale da fare a mano, per host e spettatore.

«Nessun server proprio» della specifica non regge questi requisiti: serve un punto d'incontro
sempre raggiungibile. Lo teniamo **minimo**: non vede lo stream, non tiene chiavi private, non
decide i permessi (restano sull'host, §7).

## Decisione

Un servizio minimo, **Falco Hub**, su **Cloudflare Workers + D1** ([services/hub](../../services/hub)), gratuito e
sempre acceso all'indirizzo `https://falco-hub.<sottodominio>.workers.dev`. Nessun dominio da comprare,
nessun server da mantenere.

| Cosa | Come |
| --- | --- |
| Identità | La stessa chiave Ed25519 del §4. Ogni richiesta è firmata come FPP (`x-falco-id/ts/sig`, ±60 s). Nessuna password, nessuna email. |
| Codice amico | Il tag dell'ID senza `#` (`K7QM-2XPA`), calcolato dal server dalla chiave: non si può falsificare. |
| Link amico | `https://<hub>/a/K7QM2XPA` → pagina → `falco://add/K7QM2XPA`. |
| Richiesta d'amicizia | Chi chiede allega un invito firmato a uso singolo (Spettatore, 7 giorni). Chi accetta lo riceve, il core controlla che sia firmato da chi ha chiesto e fa l'handshake del §6 come prima. |
| Link di stream | L'host crea un invito e riceve un link corto `https://<hub>/j/<codice>`. Chi lo apre accetta l'invito ed **entra subito** nella sessione. |
| Rete | Provider «Falco» (default): un'unica tailnet di Falco. Il Hub crea con l'API di Tailscale una **chiave monouso con il tag `tag:falco`** e il core fa `tailscale login --auth-key …`. Gli utenti non creano account: l'unico account Tailscale è quello di Falco (Marco). |
| Tailscale | Il client ufficiale, installato da Falco (ADR 0012, opzione A). Chi preferisce la propria tailnet sceglie il provider «Tailscale» e l'OAuth facoltativo in Impostazioni → Rete. |

### Perché Cloudflare

- **Workers + D1**: piano gratuito senza sospensione per inattività (100 000 richieste al giorno, 5 GB di D1),
  HTTPS e sottodominio inclusi. Vercel e Netlify hanno funzioni ma niente database incluso; Render e Koyeb
  sospendono i servizi gratuiti; Supabase mette in pausa i progetti fermi da una settimana.
- **Headscale scartato**: serve un processo sempre acceso con connessioni lunghe, impossibile su un servizio
  serverless gratuito. La rete la coordina Tailscale stesso (DERP e NAT traversal inclusi).

## Alternative considerate

- **Headscale su una VPS** (prima versione di questo ADR): costa e va mantenuta.
- **Account Tailscale per ogni utente + OAuth di ogni host**: il flusso di prima, troppo difficile.
- **`tsnet` incorporato**: rimandato alla misura di latenza (ADR 0012, opzione B).

## Conseguenze

- Se il Hub non risponde, gli amici già aggiunti continuano a vedersi e a giocare (presenza e stream sono
  diretti); si fermano solo nuove richieste, nuovi link e nuovi collegamenti alla rete.
- **Beta**: tutti i dispositivi `tag:falco` si raggiungono tra loro, ma solo sulle porte di Falco
  (47900–48010). Chi entra resta comunque fermato da firme FPP, pairing di Sunshine e permessi dell'host (§7).
  ACL per coppia di amici dopo la beta.
- Piano gratuito di Tailscale: fino a 100 dispositivi nella tailnet di Falco. Basta per la beta.
- Chi ha già Tailscale collegato a un'altra tailnet non viene spostato da solo: c'è «Collega alla rete Falco».
- Privacy: il Hub conosce nome, codice e chi è amico di chi. Niente avatar, niente cronologia.

## Da verificare

- [ ] **Marco**: account Cloudflare (`wrangler login`) e tailnet di Falco con OAuth client (guida in `services/hub/README.md`).
- [ ] `tailscale login --auth-key` silenzioso su Windows (`--unattended`) e macOS.
- [ ] Limiti di richieste contro abusi (regola di rate limiting di Cloudflare).
