# Pacchetto Windows

Da costruire in M6 (§14 della specifica).

- Installer NSIS per tutto il sistema, una sola richiesta UAC (da rivalutare per gli aggiornamenti silenziosi, [ADR 0008](../../docs/adr/0008-aggiornamenti-automatici.md)).
- Contenuto: `falo-core.exe`, `falo-ui.exe`, `engines\moonlight\`, `falo-sunshine` installato come servizio con avvio «manuale», gestito dal core.
- Regole firewall per `falo-sunshine.exe` limitate a `100.64.0.0/10` e `fd7a:115c:a1e0::/48` (più la LAN se il provider LAN è attivo).
- Registrazione dello schema `falo://` e avvio con il sistema nel tray.
- Firma del codice (per esempio Azure Trusted Signing).
- Tailscale non è incluso: installazione guidata dell'MSI ufficiale.
