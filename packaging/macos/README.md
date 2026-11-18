# Pacchetto macOS

Da costruire in M6 (§14 della specifica).

- `.app` universal (arm64 + x86_64) in un `.dmg`, macOS 14 o successivo (14.2 per ospitare).
- `falco-moonlight` come app di supporto in `Contents/Helpers`, firmata a parte.
- Hardened Runtime, entitlements per Registrazione schermo, Accessibilità, Rete locale e Notifiche; notarizzazione di ogni binario annidato.
- `CFBundleURLTypes` per lo schema `falco://`; elemento di login con `SMAppService`.
- Aggiornamenti: sostituzione del bundle firmato ([ADR 0008](../../docs/adr/0008-aggiornamenti-automatici.md)).
