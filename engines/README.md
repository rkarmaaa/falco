# Motori di streaming

Qui vanno i fork di Sunshine e moonlight-qt come submodule git (§3 della specifica). Non sono ancora presenti: servono i repository dei fork sotto l'organizzazione GitHub del progetto.

| Cartella | Upstream | Fork | Licenza |
| --- | --- | --- | --- |
| `sunshine/` | [LizardByte/Sunshine](https://github.com/LizardByte/Sunshine) (o [Apollo](https://github.com/ClassicOldSong/Apollo), vedi ADR 0011) | `falo-sunshine` | GPL-3.0 |
| `moonlight/` | [moonlight-stream/moonlight-qt](https://github.com/moonlight-stream/moonlight-qt) | `falo-moonlight` | GPL-3.0 |

## Aggiungere i submodule

Dopo aver creato i fork (sostituisci `ORG`):

```bash
git submodule add -b falo/main https://github.com/ORG/falo-sunshine engines/sunshine
```

```bash
git submodule add -b falo/main https://github.com/ORG/falo-moonlight engines/moonlight
```

## Regole

- Ogni modifica sta in un branch `falo/*` con commit piccoli e titolati.
- Prima di patchare, controllare se upstream offre già un'opzione o una API.
- `scripts/sync-upstream.sh` fa il rebase su upstream, compila e lancia i test di fumo.
- Patch previste: [ADR 0006](../docs/adr/0006-spike-sunshine.md) (Sunshine, permessi del §7) e [ADR 0007](../docs/adr/0007-spike-moonlight-cli.md) (Moonlight, §8.2).
