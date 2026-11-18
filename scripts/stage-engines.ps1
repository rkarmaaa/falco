# Copia i motori accanto ai binari di Falco, con lo stesso layout del pacchetto finale:
#   target\<profilo>\engines\sunshine\sunshine.exe
#   target\<profilo>\engines\moonlight\Moonlight.exe  (+ portable.dat: impostazioni separate)
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
# Moonlight portable: impostazioni nella sua cartella, non nel profilo utente
New-Item -ItemType File -Force (Join-Path $dest "moonlight\portable.dat") | Out-Null
Write-Host "ok  moonlight in modalità portable"

# Sunshine di Falco (patch in engines/patches/sunshine, scripts/build-sunshine.sh): sostituisce
# l'eseguibile ufficiale. Senza, l'audio «solo ciò che è a schermo» non è disponibile (ADR 0016).
$fork = Join-Path $root "target\sunshine\sunshine.exe"
if (Test-Path $fork) {
  Copy-Item $fork (Join-Path $dest "sunshine\sunshine.exe") -Force
  Write-Host "ok  sunshine di Falco ($fork)"
} else {
  Write-Warning "target\sunshine\sunshine.exe assente: uso Sunshine ufficiale, l'audio sarà quello di tutto il PC. Compila con scripts/build-sunshine.sh"
}
