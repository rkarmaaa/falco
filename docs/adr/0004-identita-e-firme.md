# 0004 · Identità, JSON canonico e codifiche

- **Stato**: accettato
- **Data**: 2026-11-17
- **Sezioni della specifica**: §4, §6

## Decisione

- Firme Ed25519 con `ed25519-dalek`; ID = chiave pubblica in base32 senza padding.
- Tag `#XXXX-XXXX`: primi 8 caratteri del base32 dello SHA-256 dei 32 byte della chiave pubblica.
- Accento: tonalità dai primi due byte dello SHA-256, saturazione 62%, luminosità 56%.
- JSON canonico secondo **RFC 8785** con `serde_json_canonicalizer`, indipendente dalle feature di `serde_json` attive nel workspace.
- Firme e token in base64url senza padding.
- Lo stato del profilo sul filo è in inglese (`online`, `busy`, `invisible`), come nel `config.toml`; il §4 della specifica è stato allineato. I ruoli restano in italiano (`spettatore`, `giocatore`, `copilota`, `fidato`), come nel payload d'invito.
- Inviti: JSON firmato → zstd livello 19 → base64url, prefisso `falco://invite/`; decompressione limitata a 16 KB contro le «bombe» di compressione.

## Conseguenze

Il formato di tag e accento è ora parte del protocollo: cambiarlo romperebbe la cache degli amici.
