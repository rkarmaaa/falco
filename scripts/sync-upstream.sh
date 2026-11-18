#!/usr/bin/env bash
# Riallinea i fork dei motori a upstream: rebase dei branch falco/*, build, test di fumo.
# Uso: scripts/sync-upstream.sh [sunshine|moonlight]   (senza argomenti: entrambi)
set -euo pipefail

root="$(cd "$(dirname "$0")/.." && pwd)"
declare -A upstream=(
  [sunshine]="https://github.com/LizardByte/Sunshine.git"
  [moonlight]="https://github.com/moonlight-stream/moonlight-qt.git"
)

sync_engine() {
  local name="$1" dir="$root/engines/$1"
  if [[ ! -d "$dir/.git" && ! -f "$dir/.git" ]]; then
    echo "engines/$name non è ancora un submodule: vedi engines/README.md" >&2
    return 1
  fi
  echo "==> $name"
  git -C "$dir" remote get-url upstream >/dev/null 2>&1 \
    || git -C "$dir" remote add upstream "${upstream[$name]}"
  git -C "$dir" fetch upstream --tags
  local base
  base="$(git -C "$dir" describe --tags --abbrev=0 upstream/HEAD 2>/dev/null || echo upstream/HEAD)"
  git -C "$dir" rebase "$base"
  echo "    rebase su $base completato; esegui build e test di fumo prima del push"
}

targets=("${@:-sunshine moonlight}")
for t in ${targets[@]}; do
  sync_engine "$t"
done
