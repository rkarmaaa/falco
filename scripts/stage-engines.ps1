# Copia i motori accanto ai binari di Falco, con lo stesso layout del pacchetto finale:
#   target\<profilo>\engines\sunshine\sunshine.exe
#   target\<profilo>\engines\moonlight\Moonlight.exe
# In sviluppo la sorgente sono le installazioni ufficiali; in CI saranno i fork compilati.
# Uso: powershell -File scripts\stage-engines.ps1 [-Profile debug|release]
param(
  [string]$Profile = "debug",
  [string]$SunshineDir = "C:\Program Files\Sunshine",
  [string]$MoonlightDir = "C:\Program Files\Moonlight Game Streaming"
)
$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot
$dest = Join-Path $root "target\$Profile\engines"

foreach ($pair in @(@($SunshineDir, "sunshine", "sunshine.exe"), @($MoonlightDir, "moonlight", "Moonlight.exe"))) {
  $src, $name, $exe = $pair
  if (-not (Test-Path (Join-Path $src $exe))) { throw "Motore non trovato: $src\$exe" }
  $out = Join-Path $dest $name
  New-Item -ItemType Directory -Force $out | Out-Null
  # Solo i file del programma: niente config, log o credenziali dell'installazione utente
  robocopy $src $out /E /NFL /NDL /NJH /NJS /XD config | Out-Null
  if ($LASTEXITCODE -ge 8) { throw "Copia di $name non riuscita ($LASTEXITCODE)" }
  Write-Host "ok  $name -> $out"
}
# Moonlight portable: il core lo avvia con una cartella di lavoro dell'utente che contiene
# portable.dat (%LOCALAPPDATA%\Falco\...\data\moonlight), non accanto all'exe in Program Files
Remove-Item -Force -ErrorAction SilentlyContinue (Join-Path $dest "moonlight\portable.dat")

# Sunshine di Falco (patch in engines/patches/sunshine, scripts/build-sunshine.sh): sostituisce
# l'eseguibile ufficiale. Senza, l'audio «solo ciò che è a schermo» non è disponibile (ADR 0016).
$fork = Join-Path $root "target\sunshine\sunshine.exe"
if (Test-Path $fork) {
  Copy-Item $fork (Join-Path $dest "sunshine\sunshine.exe") -Force
  Write-Host "ok  sunshine di Falco ($fork)"
} else {
  Write-Warning "target\sunshine\sunshine.exe assente: uso Sunshine ufficiale, l'audio sarà quello di tutto il PC. Compila con scripts/build-sunshine.sh"
}

# ViGEmBus: driver dei controller virtuali (Xbox 360 / DualShock 4) che Sunshine usa per i gamepad
# degli ospiti. Installer ufficiale firmato da Nefarius (licenza BSD-3), scaricato una volta e
# verificato con lo SHA-256 fissato qui; l'installer di Falco lo installa in silenzio se manca.
$vigemVersion = "1.22.0"
$vigemSha256 = "89220A7865076B342892F98865F3499FB7C4CFD673159E89D352C360FD014C6A"
$vigemCache = Join-Path $root "target\vigembus\ViGEmBus_${vigemVersion}_x64_x86_arm64.exe"
if (-not (Test-Path $vigemCache)) {
  New-Item -ItemType Directory -Force (Split-Path $vigemCache) | Out-Null
  $url = "https://github.com/nefarius/ViGEmBus/releases/download/v$vigemVersion/ViGEmBus_${vigemVersion}_x64_x86_arm64.exe"
  Invoke-WebRequest -UseBasicParsing -Uri $url -OutFile $vigemCache
}
if ((Get-FileHash $vigemCache -Algorithm SHA256).Hash -ne $vigemSha256) {
  Remove-Item -Force $vigemCache
  throw "ViGEmBus scaricato non corrisponde allo SHA-256 atteso: file rifiutato"
}
$vigemOut = Join-Path $dest "vigembus"
New-Item -ItemType Directory -Force $vigemOut | Out-Null
Copy-Item $vigemCache (Join-Path $vigemOut "ViGEmBus-setup.exe") -Force
Write-Host "ok  vigembus $vigemVersion (SHA-256 verificato)"
