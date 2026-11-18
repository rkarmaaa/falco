<p align="center">
  <img src="assets/brand/png/falco-256.png" width="112" alt="Falco" />
</p>

<h1 align="center">Falco</h1>

<p align="center">
  <strong>Gioca con gli amici. Veloce come un falco.</strong><br/>
  Streaming di gioco tra amici, senza server di mezzo.<br/>
  Un'app leggera per Windows 11 e macOS che unisce <a href="https://github.com/LizardByte/Sunshine">Sunshine</a>, <a href="https://github.com/moonlight-stream/moonlight-qt">Moonlight</a> e <a href="https://tailscale.com">Tailscale</a>.
</p>

<p align="center">
  <a href="LICENSE"><img alt="Licenza GPL-3.0" src="https://img.shields.io/badge/licenza-GPL--3.0-32409A"></a>
  <img alt="Stato: M3" src="https://img.shields.io/badge/stato-M3%20inviti-1A1E26">
</p>

---

## Cos'è

Falco è un orchestratore: configura e avvia motori di streaming maturi (Sunshine come host, Moonlight come client) e usa Tailscale come rete privata cifrata. Sopra aggiunge quello che manca per giocare tra amici come su Parsec: **profili, lista amici, inviti con un link e permessi parziali** (chi può usare controller, tastiera, mouse).

- **Un'unica app**: scarichi solo Falco, crei il profilo, inviti un amico e giocate. Niente da installare o configurare a mano: motori e rete arrivano con Falco.
- **Zero overhead in gioco**: durante lo streaming la finestra di Falco non esiste; resta solo un demone da pochi MB.
- **Nessun server nostro**: i dati tra amici viaggiano peer-to-peer dentro la tailnet.
- **Sicuro per costruzione**: nessuna porta esposta su Internet, ogni messaggio tra amici è firmato, i permessi si applicano sull'host.

> [!NOTE]
> Il progetto è alla milestone **M3 · Inviti**: Falco fa da host e da client, con profili firmati, amici, presenza, inviti con un link e pairing automatico. Permessi granulari, rete avanzata e pacchetto di installazione arrivano con M4–M6. Vedi [TODO.md](TODO.md).

## Documentazione

| Documento | Contenuto |
| --- | --- |
| [docs/falco-spec.md](docs/falco-spec.md) | **Specifica completa**, fonte di verità: architettura, protocolli, sicurezza, roadmap |
| [DESIGN_SYSTEM.md](DESIGN_SYSTEM.md) | Colori, tipografia, componenti, regole SCSS |
| [TODO.md](TODO.md) | Stato delle milestone e lavoro aperto |
| [docs/ipc.md](docs/ipc.md) | Contratto JSON-RPC tra core e UI |
| [docs/adr/](docs/adr/) | Decisioni di architettura (ADR) |
| [CONTRIBUTING.md](CONTRIBUTING.md) | Convenzioni, commit, fork dei motori |
| [SECURITY.md](SECURITY.md) | Come segnalare una vulnerabilità |

## Architettura in breve

```mermaid
flowchart LR
  UI["falco-ui<br/>Tauri 2 + Vue 3<br/>(solo a finestra aperta)"] -- "JSON-RPC<br/>named pipe / socket" --> CORE["falco-core<br/>Rust, tray"]
  CORE --> SUN["falco-sunshine<br/>(host)"]
  CORE --> MOON["falco-moonlight<br/>(ospite)"]
  CORE --> TS["Tailscale CLI"]
  CORE <-. "presenza FPP<br/>via tailnet" .-> FRIEND["core di un amico"]
```

## Struttura del repository

```text
apps/
  core/            demone Rust: tray, IPC, orchestrazione
  ui/              Tauri 2 + Vue 3 + SCSS, avviata su richiesta
crates/
  falco-proto/      identità Ed25519, profili firmati, permessi, inviti, schema IPC
  falco-net/        provider di rete (Tailscale) e Dottore di rete
  falco-store/      config.toml validato e SQLite con migrazioni
  falco-engines/    sunshine.conf, argomenti di Moonlight, processi
engines/           fork di Sunshine e Moonlight (submodule, da aggiungere)
packaging/         installer Windows e bundle macOS
docs/              specifica, ADR, guide
scripts/           sviluppo, test di fumo, sync dei fork
```

## Sviluppo

Requisiti: Rust stable (1.85+), Node.js 20+, su Windows i Build Tools di Visual Studio con il carico «Sviluppo di applicazioni desktop C++» e WebView2 (già presente in Windows 11). Tailscale serve per le funzioni di rete.

```bash
npm --prefix apps/ui install
```

Test e controlli:

```bash
cargo test --workspace
```

```bash
cargo clippy --workspace --all-targets -- -D warnings
```

```bash
npm --prefix apps/ui run typecheck && npm --prefix apps/ui test
```

Motori in sviluppo: copia Sunshine e Moonlight installati accanto ai binari, con lo stesso layout del pacchetto finale.

```bash
powershell -File scripts/stage-engines.ps1
```

Avvio in sviluppo: prima il core (icona nel tray), poi la finestra.

```bash
cargo run -p falco-core
```

```bash
cd apps/ui && npx tauri dev
```

Solo interfaccia nel browser, con un core simulato:

```bash
npm --prefix apps/ui run dev
```

Con il core avviato, `node scripts/ipc-smoke.mjs` prova l'IPC, `node scripts/rpc.mjs <metodo> [json]` chiama un metodo e `node scripts/e2e-loopback.mjs [porta]` esegue la prova completa di M1 su un solo PC. `falco-core bench 60 tray` misura RAM e CPU.

## Licenza

Falco è software libero sotto licenza [GPL-3.0](LICENSE), come Sunshine e Moonlight da cui deriva. Tailscale è il client ufficiale, installato da Falco in automatico; ognuno usa il proprio account. Nome, logo e icone sono originali.
