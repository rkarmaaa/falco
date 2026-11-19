#!/usr/bin/env bash
# Compila il Sunshine di Falco: upstream al commit fissato + le patch in engines/patches/sunshine.
# Gira nella shell MSYS2 UCRT64 (Windows), come richiede la build ufficiale di Sunshine:
#   C:\msys64\msys2_shell.cmd -defterm -here -no-start -ucrt64 -c "scripts/build-sunshine.sh"
# Risultato: target/sunshine/sunshine.exe, usato da scripts/stage-engines.ps1.
set -euo pipefail

root="$(cd "$(dirname "$0")/.." && pwd)"
commit="63d35f702ee9e362e43263742981836ec0710384"   # Sunshine 2026.914.233613
src="${FALCO_SUNSHINE_SRC:-$root/../falco-sunshine}"
patches="$root/engines/patches/sunshine"
out="$root/target/sunshine"

deps=(git cmake ninja boost cppwinrt curl-winssl miniupnpc onevpl openssl opus toolchain MinHook)
missing=()
for d in "${deps[@]}"; do
  [[ "$d" == git ]] && pkg=git || pkg="mingw-w64-ucrt-x86_64-$d"
  pacman -Q "$pkg" >/dev/null 2>&1 || missing+=("$pkg")
done
if ((${#missing[@]})); then
  pacman -S --needed --noconfirm "${missing[@]}"
fi

if [[ ! -d "$src/.git" ]]; then
  git clone --filter=blob:none https://github.com/LizardByte/Sunshine.git "$src"
fi
git -C "$src" fetch --quiet origin "$commit" || true
if ! git -C "$src" rev-parse --verify --quiet falco/main >/dev/null; then
  git -C "$src" checkout -b falco/main "$commit"
  git -C "$src" -c user.name=falco -c user.email=falco@localhost am "$patches"/*.patch
fi
git -C "$src" checkout --quiet falco/main
git -C "$src" submodule update --init --recursive --depth 1 --jobs 8

# Node nativo per il frontend (quello di MSYS2 non è supportato da Sunshine)
export PROCESSOR_ARCHITECTURE="${PROCESSOR_ARCHITECTURE:-AMD64}"
# Versione mostrata nei log e ai client: quella di upstream, con il ramo di Falco
export BRANCH=falco BUILD_VERSION=2026.914.233613 COMMIT="$commit"
export PATH="/c/Program Files/nodejs:$PATH"
cmake -B "$src/cmake-build-release" -G Ninja -S "$src" -DCMAKE_BUILD_TYPE=Release \
  -DBUILD_DOCS=OFF -DBUILD_TESTS="${FALCO_SUNSHINE_TESTS:-OFF}" -DSUNSHINE_ENABLE_TRAY=OFF -DDOTNET_EXECUTABLE=OFF
# Con troppi processi paralleli MinGW su Windows a volte fallisce con «Invalid argument» sugli
# include (file bloccati dall'antivirus): pochi job e un secondo tentativo.
build() { ninja -C "$src/cmake-build-release" -j "${FALCO_JOBS:-8}" "$@" || ninja -C "$src/cmake-build-release" -j 4 "$@"; }
build sunshine
if [[ "${FALCO_SUNSHINE_TESTS:-OFF}" == ON ]]; then
  build test_sunshine
  "$src/cmake-build-release/tests/test_sunshine.exe" --gtest_filter='AudioOnscreenTest.*'
fi

mkdir -p "$out"
cp "$src/cmake-build-release/sunshine.exe" "$out/sunshine.exe"
echo "ok  $out/sunshine.exe"
