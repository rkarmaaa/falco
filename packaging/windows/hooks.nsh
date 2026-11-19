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
  ; Sunshine crea all'avvio la cartella config accanto al proprio .exe. Il core gira come utente
  ; normale e non può scrivere in Program Files: la crea l'installer (basta che esista, i dati
  ; veri stanno in %LOCALAPPDATA%\Falco)
  CreateDirectory "$INSTDIR\engines\sunshine\config"
  ; Controller degli ospiti: driver ViGEmBus in silenzio, solo se il servizio non c'è già.
  ; Stessa conferma di Windows dell'installer di Falco, nessuna finestra in più.
  ClearErrors
  ReadRegStr $0 HKLM "SYSTEM\CurrentControlSet\Services\ViGEmBus" "ImagePath"
  IfErrors 0 falco_vigem_done
  IfFileExists "$INSTDIR\engines\vigembus\ViGEmBus-setup.exe" 0 falco_vigem_done
  ExecWait '"$INSTDIR\engines\vigembus\ViGEmBus-setup.exe" /exenoui /qn /norestart'
  falco_vigem_done:
  ; Streaming in ingresso solo dagli intervalli di Tailscale (100.64.0.0/10, fd7a:115c:a1e0::/48)
  nsExec::Exec 'netsh advfirewall firewall delete rule name="Falco Sunshine"'
  nsExec::Exec 'netsh advfirewall firewall add rule name="Falco Sunshine" dir=in action=allow program="$INSTDIR\engines\sunshine\sunshine.exe" remoteip=100.64.0.0/10,fd7a:115c:a1e0::/48 enable=yes'
  nsExec::Exec 'netsh advfirewall firewall delete rule name="Falco Presence"'
  nsExec::Exec 'netsh advfirewall firewall add rule name="Falco Presence" dir=in action=allow program="$INSTDIR\falco-core.exe" remoteip=100.64.0.0/10,fd7a:115c:a1e0::/48 enable=yes'
  ; Avvio del core nel tray: registra da solo avvio con il sistema e schema falco://.
  ; Come utente normale, non con i privilegi dell'installer: un core da amministratore
  ; crea una pipe che la finestra di Falco (utente normale) non può aprire (EPERM).
  nsis_tauri_utils::RunAsUser "$INSTDIR\falco-core.exe" "--autostart"
!macroend

!macro NSIS_HOOK_PREUNINSTALL
  !insertmacro FALCO_KILL
!macroend

!macro NSIS_HOOK_POSTUNINSTALL
  nsExec::Exec 'netsh advfirewall firewall delete rule name="Falco Sunshine"'
  nsExec::Exec 'netsh advfirewall firewall delete rule name="Falco Presence"'
  RMDir "$INSTDIR\engines\sunshine\config"
  DeleteRegKey HKCU "Software\Classes\falco"
  DeleteRegValue HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "Falco"
!macroend
