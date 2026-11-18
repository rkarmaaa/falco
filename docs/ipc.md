# Contratto IPC core ↔ UI

JSON-RPC 2.0, un messaggio JSON per riga, su named pipe `\\.\pipe\falo-core-<utente>` (Windows) o socket Unix `core.sock` nella cartella dati locale (macOS). Decisione: [ADR 0002](adr/0002-ipc-json-rpc.md). Tipi Rust in [`crates/falo-proto/src/ipc.rs`](../crates/falo-proto/src/ipc.rs), tipi TypeScript in [`apps/ui/src/ipc/types.ts`](../apps/ui/src/ipc/types.ts).

```json
→ {"jsonrpc":"2.0","id":1,"method":"host.setAvailable","params":{"available":true}}
← {"jsonrpc":"2.0","id":1,"result":{"available":true}}
← {"jsonrpc":"2.0","method":"presence.changed","params":{"self":{"hostAvailable":true}}}
```

## Metodi

| Metodo | Parametri | Risultato | Stato |
| --- | --- | --- | --- |
| `state.snapshot` | — | `StateSnapshot`: versione, milestone, stato, host disponibile, streaming, rete, numero di amici | M0 |
| `config.get` | — | `{ config }` con tutte le sezioni del §15 | M0 |
| `config.set` | `{ key: "sezione.chiave", value }` | `{ applyAt: "now" \| "nextSession" \| "hostRestart" }` | M0 |
| `config.reset` | `{ section }` | `null` | M0 |
| `profile.update` | `{ status? }` (nome e avatar in M2) | `null` | M0 parziale |
| `host.setAvailable` | `{ available }` | `{ available }`; avvia o ferma Sunshine, avanzamento con `host.state` | M1 |
| `net.runDoctor` | — | `{ checks: [{ id, level, value, action }] }` | M0 parziale |
| `app.openUi` | — | `null`; usato dalla seconda istanza del core | M0 |
| `host.pairings` | — | richieste di pairing in attesa `[{ id, name, address }]` | M1 |
| `host.approvePairing` | `{ id, pin, name }` | `null` | M1 |
| `host.clients` | — | dispositivi associati `[{ uuid, name, enabled }]` | M1 |
| `host.unpair` | `{ uuid }` | `null` | M1 |
| `host.setClientEnabled` | `{ uuid, enabled }` | `null` | M1 |
| `host.endSession` | — | `null` | M1 |
| `session.pair` | `{ host, port? }` | `{ pin, host }`; esito con l'evento `pair.result` | M1 |
| `session.join` | `{ host, port?, app? }` | `null`; la UI si chiude, lo stato arriva con `session.stats` e `session.ended` | M1 |
| `profile.get` | — | `{ exists, id, tag, accent, name, avatarUrl }` | M2 |
| `profile.create` | `{ name }` | `null` | M2 |
| `profile.update` | `{ name?, status?, avatarBase64?, clearAvatar? }` | `null` | M2 |
| `friends.list` | — | amici con stato live, dispositivi e ruolo | M2 |
| `friends.refresh` | — | `null`; aggiorna subito la presenza | M2 |
| `friend.favorite` | `{ id, favorite }` | `null` | M2 |
| `friend.remove`, `friend.block` | `{ id }` | `null` | M2 |
| `invite.create` | `{ role, validityHours, uses }` | `{ link, expiresAt, tailscaleShared }` | M3 |
| `invite.list` | — | inviti con usi, scadenza, stato | M3 |
| `invite.revoke` | `{ tokenHash }` | `null` | M3 |
| `invite.preview` | `{ link }` | host, ruolo, permessi, scadenza | M3 |
| `invite.accept` | `{ link }` | `null`; avanzamento con `invite.progress` | M3 |
| `invite.approve` | `{ requestId, accept }` | `null` | M3 |
| `tailscale.setOAuth` | `{ clientId, clientSecret }` | `null` (verifica e salva nel portachiavi) | M3 |
| `friend.setPermissions` | — | — | M4 |
| `update.check`, `update.install` | — | — | M6 ([ADR 0008](adr/0008-aggiornamenti-automatici.md)) |

## Eventi

| Evento | Parametri | Stato |
| --- | --- | --- |
| `presence.changed` | `{ self: { status?, hostAvailable? } }`; in M2 anche gli amici | M0 |
| `config.changed` | `{ key, value }` oppure `{ section }` | M0 |
| `invite.progress` | `{ side: guest \| host, step, ok, message? }` | M3 |
| `invite.request` | `{ requestId, name, tag }` | M3 |
| `host.state` | `{ state: stopped \| starting \| ready \| error, message? }` | M1 |
| `pair.result` | `{ host, ok }` | M1 |
| `session.stats` | `{ phase: connecting \| connected \| stats, rttMs?, fps?, bitrateKbps?, path? }` | M1 |
| `session.ended` | `{ host, reason, durationS }` | M1 |
| `session.knock` | — | M4 |
| `net.warning` | — | M5 |
| `update.available` | — | M6 |

## Codici di errore

| Codice | Significato | `data` |
| --- | --- | --- |
| `-32700` | JSON non valido | — |
| `-32601` | Metodo sconosciuto | — |
| `-32602` | Parametri non validi | — |
| `-32603` | Errore interno | — |
| `-32001` | Previsto dalla specifica, non ancora implementato | `{ milestone }` |
| `-32002` | Valore di configurazione non valido | `{ key, value, allowed, line }` |
| `-32003` | L'host non è attivo | — |
| `-32004` | Sessione non avviata | — |
| `-32099` | Solo lato UI: core non raggiungibile | — |

## Prova da riga di comando

Con il core avviato:

```bash
node scripts/ipc-smoke.mjs
```
