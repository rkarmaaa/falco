# Costruisce il pacchetto di Falco per Windows: un solo installer con core, UI e motori.
#   1. falco-core in release (LTO), copiato come sidecar di Tauri
#   2. motori in target\release\engines (stage-engines.ps1; in CI i fork compilati)
#   3. tauri build → installer NSIS in target\release\bundle\nsis
# Uso: powershell -File scripts\package.ps1 [-SkipEngines] [-HubUrl https://falco-hub.<sottodominio>.workers.dev]
param([switch]$SkipEngines, [string]$HubUrl = $env:FALCO_HUB_URL)
$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot
Set-Location $root

# Falco Hub incluso nella build (ADR 0017); senza, amici per codice e link restano spenti
if ($HubUrl) { $env:FALCO_HUB_URL = $HubUrl; Write-Host "==> Falco Hub: $HubUrl" }
else { Write-Warning "FALCO_HUB_URL non impostato: build senza Falco Hub" }

Write-Host "==> falco-core (release)"
cargo build --release -p falco-core
if ($LASTEXITCODE -ne 0) { throw "build del core non riuscita" }

$triple = (rustc -vV | Select-String "^host:").ToString().Split(" ")[1]
$bin = Join-Path $root "apps\ui\src-tauri\binaries"
New-Item -ItemType Directory -Force $bin | Out-Null
Copy-Item "target\release\falco-core.exe" (Join-Path $bin "falco-core-$triple.exe") -Force

if (-not $SkipEngines) {
  Write-Host "==> motori"
  & powershell -NoProfile -ExecutionPolicy Bypass -File (Join-Path $PSScriptRoot "stage-engines.ps1") -Profile release
}

Write-Host "==> interfaccia e installer"
Push-Location "apps\ui"
# npm ci solo se le dipendenze sono cambiate: reinstallare tutto ogni volta costa minuti
$stamp = "node_modules\.package-lock.json"
if (-not (Test-Path $stamp) -or (Get-Item "package-lock.json").LastWriteTime -gt (Get-Item $stamp).LastWriteTime) {
  npm ci
} else {
  Write-Host "    dipendenze della UI già aggiornate"
}
npx tauri build --config src-tauri/tauri.bundle.json
$code = $LASTEXITCODE
Pop-Location
if ($code -ne 0) { throw "tauri build non riuscito" }

Get-ChildItem "target\release\bundle\nsis\*.exe" | ForEach-Object {
  "{0}  {1:N1} MB" -f $_.FullName, ($_.Length / 1MB)
}
