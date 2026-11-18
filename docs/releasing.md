# Rilasciare Falco

## Chiave degli aggiornamenti

Generata il 2026-11-18 con `npx tauri signer generate`. **Non è nel repository.**

| File | Dove | Contenuto |
| --- | --- | --- |
| `falco-updater.key` | `%USERPROFILE%\.falco\` | Chiave privata minisign (cifrata con password) |
| `falco-updater.password.txt` | `%USERPROFILE%\.falco\` | Password della chiave |
| `falco-updater.key.pub` | `%USERPROFILE%\.falco\` | Chiave pubblica, copiata in `apps/core/src/updater.rs` (`PUBLIC_KEY`) |

La cartella è accessibile solo al tuo utente. **Fai una copia di sicurezza** della chiave e della password (per esempio in un gestore di password): se si perdono, gli utenti già installati non potranno più ricevere aggiornamenti automatici e dovranno reinstallare.

Per il workflow di rilascio servono due segreti nel repository GitHub (*Settings → Secrets and variables → Actions*):

| Segreto | Valore |
| --- | --- |
| `TAURI_SIGNING_PRIVATE_KEY` | Il contenuto del file `falco-updater.key` |
| `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` | Il contenuto di `falco-updater.password.txt` |

Con `gh`, dalla cartella del repository:

```bash
gh secret set TAURI_SIGNING_PRIVATE_KEY < ~/.falco/falco-updater.key
```

```bash
gh secret set TAURI_SIGNING_PRIVATE_KEY_PASSWORD < ~/.falco/falco-updater.password.txt
```

## Repository

Account personale `rkarmaaa`, repository `rkarmaaa/falco` (nessuna organizzazione). L'URL del manifest degli aggiornamenti è `https://github.com/rkarmaaa/falco/releases/latest/download/latest.json`: se il repository cambia nome o proprietario va aggiornato in `updater.rs`.

## Pubblicare una versione

1. Aggiorna `version` in `Cargo.toml`, `apps/ui/package.json`, `apps/ui/src-tauri/tauri.conf.json` e il `CHANGELOG.md`.
2. Crea il tag e fai push: `git tag v1.0.0 && git push origin v1.0.0`.
3. `release.yml` costruisce i pacchetti, li firma per l'updater e pubblica `latest.json`.

### Beta senza CI (come la 1.0.0-rc.2)

Finché i segreti del workflow non sono impostati, si rilascia da questo PC:

1. `powershell -File scripts/package.ps1 -HubUrl https://falco-hub.luccimarco176.workers.dev`
2. Firma per l'updater (da `apps/ui`, la password si legge dal file, non va scritta a mano):
   `TAURI_SIGNING_PRIVATE_KEY_PASSWORD="$(cat ~/.falco/falco-updater.password.txt)" npx.cmd tauri signer sign -f ~/.falco/falco-updater.key <installer>.exe`
3. `latest.json` con `version`, `notes` e `platforms.windows-x86_64` = `{ signature: <contenuto del .sig>, url: <link dell'installer nella release> }`.
4. `gh release create beta-<versione> <installer>.exe <installer>.exe.sig latest.json --latest --title "Falco 1.0 Beta (<versione>)"`.
   Il tag non inizia con `v`, così non parte `release.yml`. La release deve essere **latest**, non pre-release: l'updater legge `releases/latest/download/latest.json`.

Il sito `https://falco-hub.luccimarco176.workers.dev/download` porta sempre all'installer dell'ultima release.

Prova locale dell'installer Windows:

1. Sunshine di Falco (patch in `engines/patches/sunshine`, [ADR 0016](adr/0016-audio-solo-a-schermo.md)). Serve [MSYS2](https://www.msys2.org) in `C:\msys64`; lo script installa da solo le dipendenze. La prima volta richiede circa 20 minuti.

   ```bash
   C:/msys64/msys2_shell.cmd -defterm -here -no-start -ucrt64 -c scripts/build-sunshine.sh
   ```

2. Installer: `powershell -File scripts/package.ps1`. Se manca `target/sunshine/sunshine.exe` il pacchetto usa il Sunshine ufficiale e lo segnala: in quel caso l'audio è quello di tutto il PC.

## Senza firma del codice

Decisione del 2026-11-18: niente certificato di firma del codice (budget). Conseguenze:

| Dove | Effetto | Mitigazione |
| --- | --- | --- |
| Windows, primo download dal sito | SmartScreen mostra «Windows ha protetto il PC»: l'utente preme *Ulteriori informazioni → Esegui comunque* | Spiegarlo nella pagina di download, con uno screenshot |
| Windows, aggiornamenti automatici | Nessun avviso: il core scarica l'installer senza «Mark of the Web» e ne verifica la **firma minisign** prima di eseguirlo | — |
| macOS | Gatekeeper blocca l'app non notarizzata: tasto destro → *Apri*, oppure *Impostazioni → Privacy e sicurezza → Apri comunque* | Spiegarlo nella pagina di download |
| Integrità | La firma minisign degli aggiornamenti protegge comunque da pacchetti manomessi | — |

La firma del codice resta consigliata quando ci sarà budget: Azure Trusted Signing costa circa 10 € al mese.
