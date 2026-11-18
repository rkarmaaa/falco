; Hook dell'installer NSIS di Falco (Tauri 2, bundle.windows.nsis.installerHooks).
; Regole firewall limitate alla tailnet (§9), chiusura dei processi prima di aggiornare.

!macro FALCO_KILL
  nsExec::Exec 'taskkill /F /T /IM falco-core.exe'
  nsExec::Exec 'taskkill /F /T /IM falco-ui.exe'
  ; /T chiude anche il Sunshine avviato dal core, senza toccare altri Sunshine del sistema
!macroend

!macro NSIS_HOOK_PREINSTALL
  !insertmacro FALCO_KILL
!macroend

!macro NSIS_HOOK_POSTINSTALL
  ; Streaming in ingresso solo dagli intervalli di Tailscale (100.64.0.0/10, fd7a:115c:a1e0::/48)
  nsExec::Exec 'netsh advfirewall firewall delete rule name="Falco Sunshine"'
  nsExec::Exec 'netsh advfirewall firewall add rule name="Falco Sunshine" dir=in action=allow program="$INSTDIR\engines\sunshine\sunshine.exe" remoteip=100.64.0.0/10,fd7a:115c:a1e0::/48 enable=yes'
  nsExec::Exec 'netsh advfirewall firewall delete rule name="Falco Presence"'
  nsExec::Exec 'netsh advfirewall firewall add rule name="Falco Presence" dir=in action=allow program="$INSTDIR\falco-core.exe" remoteip=100.64.0.0/10,fd7a:115c:a1e0::/48 enable=yes'
  ; Avvio del core nel tray: registra da solo avvio con il sistema e schema falco://
  Exec '"$INSTDIR\falco-core.exe" --autostart'
!macroend

!macro NSIS_HOOK_PREUNINSTALL
  !insertmacro FALCO_KILL
!macroend

!macro NSIS_HOOK_POSTUNINSTALL
  nsExec::Exec 'netsh advfirewall firewall delete rule name="Falco Sunshine"'
  nsExec::Exec 'netsh advfirewall firewall delete rule name="Falco Presence"'
  DeleteRegKey HKCU "Software\Classes\falco"
  DeleteRegValue HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "Falco"
!macroend
