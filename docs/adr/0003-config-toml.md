# 0003 · config.toml senza riscritture distruttive

- **Stato**: accettato
- **Data**: 2026-11-17
- **Sezioni della specifica**: §15

## Decisione

- `toml_edit` per le modifiche: commenti, ordine e **chiavi sconosciute restano**. La lettura tipizzata usa `serde` con un default per ogni campo, così un file parziale è valido.
- Validazione in due passi: tipi ed enumerazioni (errore con numero di riga dal parser), poi intervalli e valori ammessi (errore con chiave, valore, intervallo e riga).
- `set` lavora su una copia del documento: se la validazione fallisce, né il file né lo stato cambiano. Scrittura atomica con `tempfile` (file temporaneo e rename).
- Il commento in linea di una chiave modificata viene conservato.
- Ogni chiave ha un momento di applicazione (`now`, `nextSession`, `hostRestart`) restituito da `config.set`.
- `advanced.sunshine_overrides.*` accetta chiavi libere.
- Aggiunta rispetto alla specifica originale: `general.updates = "auto" | "notify" | "off"` (ADR 0008).
