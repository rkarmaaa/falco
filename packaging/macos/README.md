# Pacchetto macOS

`.app` e `.dmg` di Tauri 2 (workflow `release.yml`, `--bundles app,dmg`), macOS 14 o successivo.

- `Info.plist`: schema `falco://`, servizio Bonjour `_falco._tcp`, spiegazioni per Rete locale e Registrazione schermo.
- `entitlements.plist`: rete client e server, audio, Apple Events.
- Da fare: Moonlight come app di supporto in `Contents/Helpers` firmata a parte, Hardened Runtime e notarizzazione (serve Apple Developer Program).
- Aggiornamenti: `.app.tar.gz` firmato, estratto al posto del bundle (ADR 0008).
