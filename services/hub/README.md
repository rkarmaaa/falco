# Falco Hub · Cloudflare Workers

Servizio di [ADR 0017](../../docs/adr/0017-falco-hub.md): codici amico, richieste d'amicizia, link di stream e
chiavi monouso della rete Falco. Gratis, sempre acceso, su `https://falco-hub.<sottodominio>.workers.dev`.

## Pubblicazione (una volta)

```powershell
cd services/hub
npm ci
npx wrangler login                 # account Cloudflare gratuito, nel browser
npx wrangler d1 create falco-hub   # copia database_id in wrangler.toml
npx wrangler deploy                # stampa l'indirizzo del Worker
```

Lo schema del database si crea da solo al primo uso.

## Rete Falco senza account (una volta)

Gli utenti entrano in un'unica tailnet di Falco con chiavi monouso: l'unico account Tailscale è questo.

1. Crea un account gratuito su <https://login.tailscale.com>.
2. **Access controls** → aggiungi al file della politica:

   ```jsonc
   "tagOwners": { "tag:falco": ["autogroup:admin"] },
   "grants": [
     // Solo le porte di Falco: presenza 47900, Sunshine 47984–48010
     { "src": ["tag:falco"], "dst": ["tag:falco"], "ip": ["tcp:47900-48010", "udp:47900-48010"] }
   ]
   ```

   e togli la regola predefinita che lascia passare tutto, se non serve ad altri dispositivi.
3. **Settings → Trust credentials → OAuth client**: scope **Auth Keys: write**, tag `tag:falco`.
4. Salva ID e secret nel Worker (li digiti tu, non finiscono nel repository):

   ```powershell
   npx wrangler secret put TS_OAUTH_ID
   npx wrangler secret put TS_OAUTH_SECRET
   ```

## Download dell'installer (repository privato)

`https://falco-hub.<sottodominio>.workers.dev/download` rimanda all'installer dell'ultima release. Il Worker
lo trova con un token GitHub **fine-grained** di sola lettura (solo il repository `falco`, permesso
*Contents: Read-only*):

```powershell
npx.cmd wrangler secret put GITHUB_TOKEN
```

## Collegare l'app

L'indirizzo entra nell'app in fase di build: `FALCO_HUB_URL=https://falco-hub.<sottodominio>.workers.dev`
(lo usa `scripts/package.ps1`). Per prove: Impostazioni → Rete → «Server Falco Hub».

## Sviluppo

`npm run dev` (Worker e D1 in locale su <http://127.0.0.1:8787>, l'indirizzo che usano le build di debug),
`npm test` (Worker vero in locale), `npm run typecheck`.
