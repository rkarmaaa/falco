# Istruzioni per agenti

- La fonte di verità è [docs/falco-spec.md](docs/falco-spec.md). Lavora una milestone alla volta (§16) seguendo il §18; lo stato è in [TODO.md](TODO.md).
- Prima di implementare elenca file da creare, decisioni aperte e punti da verificare; per scelte non banali scrivi un ADR in `docs/adr/`.
- **Preferisci librerie e API mature al codice scritto a mano**, entro i budget del §10. Giustifica le dipendenze pesanti.
- Convenzioni in [CONTRIBUTING.md](CONTRIBUTING.md); stile e componenti in [DESIGN_SYSTEM.md](DESIGN_SYSTEM.md).
- Fermati e chiedi a Marco per: test su hardware reale, account e costi (Apple Developer, firma Windows, OAuth Tailscale), approvazione delle schermate, qualsiasi cambio a permessi o sicurezza.

## Comandi

- `cargo test --workspace` · `cargo clippy --workspace --all-targets -- -D warnings` · `cargo fmt --all`
- `npm --prefix apps/ui run typecheck` · `npm --prefix apps/ui test` · `npm --prefix apps/ui run dev` (UI nel browser con core simulato)
- `cargo run -p falco-core`, poi `cd apps/ui && npx tauri dev`; `node scripts/ipc-smoke.mjs` prova l'IPC.

## Prompt di avvio per ogni milestone

```text
Leggi docs/falco-spec.md. Lavora solo sulla milestone M<N>.
Prima elenca file da creare, decisioni aperte e punti da verificare.
Poi implementa seguendo le convenzioni del §18, esegui test e falco bench,
aggiorna il documento se serve e chiudi con i criteri di accettazione spuntati
o con l'elenco preciso di cosa manca.
```
