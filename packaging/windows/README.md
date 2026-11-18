# Pacchetto Windows

Installer NSIS di Tauri 2, per tutto il sistema (una richiesta UAC, necessaria per il firewall).

```bash
powershell -File scripts/package.ps1
```

| Cosa | Dove |
| --- | --- |
| `falco-ui.exe` | Punto d'ingresso (collegamento nel menu Start); avvia il core se manca |
| `falco-core.exe` | Sidecar: tray, IPC, motori, presenza; si registra per l'avvio con il sistema e per `falco://` |
| `engines\sunshine`, `engines\moonlight` | Motori (in sviluppo copiati dalle installazioni ufficiali con `stage-engines.ps1`; in CI i fork) |
| `hooks.nsh` | Regole firewall per Sunshine e la presenza limitate a `100.64.0.0/10` e `fd7a:115c:a1e0::/48`; chiusura dei processi prima di aggiornare o disinstallare |

Configurazione: `apps/ui/src-tauri/tauri.conf.json` (installer) e `tauri.bundle.json` (sidecar e motori, solo per il pacchetto).

Da fare: firma del codice (per esempio Azure Trusted Signing), altrimenti SmartScreen avvisa al primo avvio.
