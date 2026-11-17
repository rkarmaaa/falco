# Contribuire a Falò

Grazie dell'interesse. Prima di tutto leggi la [specifica](docs/falo-spec.md): è la fonte di verità e il lavoro procede una milestone alla volta (§16), seguendo [TODO.md](TODO.md).

## Ordine di lavoro

1. Rileggi le sezioni della specifica coinvolte e scrivi un piano breve: file da creare, decisioni aperte, rischi.
2. Per ogni scelta non banale scrivi un ADR di una pagina in `docs/adr/NNNN-titolo.md` partendo da [0000-modello.md](docs/adr/0000-modello.md).
3. Implementa in quest'ordine: tipi condivisi → logica del core → IPC → UI → test → `falo bench`.
4. Esegui test e lint, aggiorna la specifica se qualcosa è cambiato, poi riassumi cosa resta.

## Convenzioni

- **Librerie prima del codice scritto a mano.** Se esiste una crate o un pacchetto npm maturo che fa il lavoro, usalo: meno codice da mantenere. Vale il limite dei budget del §10: ogni dipendenza deve giustificare il suo peso (`cargo bloat`, dimensione del bundle).
- **Rust**: `rustfmt`, `cargo clippy -- -D warnings`, nessun `unwrap()` fuori dai test (lint `clippy::unwrap_used`), errori con `thiserror` nelle librerie e `anyhow` solo ai bordi (binari).
- **TypeScript** in modalità `strict`; componenti Vue con `<script setup>`; SCSS secondo [DESIGN_SYSTEM.md](DESIGN_SYSTEM.md#regole-scss).
- Nomi di variabili, funzioni e file **in inglese**; commenti **brevi, in italiano**, massimo due righe.
- Ogni testo dell'interfaccia passa da chiavi i18n, **in italiano e in inglese**.
- Errori mostrati all'utente sempre in tre parti: cosa, perché, cosa fare.
- Fine riga LF (lo impone `.gitattributes`).

## Commit e branch

- [Conventional Commits](https://www.conventionalcommits.org/it/), piccoli e con un solo scopo: `feat(core): avvio di Sunshine come servizio`, `fix(ui): focus nella barra laterale`, `docs(adr): spike moonlight`.
- Ambiti consigliati: `core`, `ui`, `proto`, `net`, `store`, `engines`, `packaging`, `docs`, `ci`.
- Un branch per funzione; la PR elenca i criteri di accettazione toccati.

## Fork dei motori

- Ogni modifica a Sunshine o Moonlight sta in un branch `falo/*` del rispettivo fork, con commit piccoli e titolati, così il rebase su upstream resta fattibile.
- Prima di patchare, controlla se upstream offre già un'opzione di configurazione o una API.
- `scripts/sync-upstream.sh` fa il rebase, compila e lancia i test di fumo.

## Definition of done

- [ ] Test automatici scritti e verdi in CI su Windows e macOS
- [ ] Budget del §10 rispettati
- [ ] Testi in italiano e in inglese
- [ ] Uso da tastiera e focus visibile
- [ ] Errori con messaggio in tre parti
- [ ] Specifica e ADR aggiornati

## Comandi utili

```bash
cargo test --workspace
```

```bash
cargo clippy --workspace --all-targets -- -D warnings
```

```bash
cargo fmt --all
```

```bash
npm --prefix apps/ui run typecheck && npm --prefix apps/ui test
```

## Licenza

Contribuendo accetti che il tuo codice sia rilasciato sotto [GPL-3.0](LICENSE).
