#!/usr/bin/env bash
# Startup script for the cloud environment (Claude Code on the web).
# Idempotent: safe to re-run at every session start. Pass --data to also run fetch-data (1-2 GB, engine/CLI sessions only).
set -euo pipefail
cd "$(dirname "$0")/.."

fetch_data=0
for arg in "$@"; do
  case "$arg" in
    --data) fetch_data=1 ;;
    *) echo "usage: $0 [--data]" >&2; exit 2 ;;
  esac
done

# The engine is a git submodule; without it nothing in crates/ builds.
git submodule update --init --recursive

# System libraries of the Tauri backend (WebKitGTK) and Xvfb for the real app without a window.
if ! dpkg -s libwebkit2gtk-4.1-dev >/dev/null 2>&1 || ! command -v xvfb-run >/dev/null 2>&1; then
  SUDO=""; [ "$(id -u)" -ne 0 ] && SUDO="sudo"
  $SUDO apt-get update -qq
  DEBIAN_FRONTEND=noninteractive $SUDO apt-get install -y -qq --no-install-recommends \
    libwebkit2gtk-4.1-dev libgtk-3-dev libayatana-appindicator3-dev librsvg2-dev libxdo-dev xauth xvfb
fi

(cd apps/desktop && pnpm install --frozen-lockfile)
cargo fetch --locked

if [ "$fetch_data" -eq 1 ]; then
  cargo run --release -p ygo-cli -- fetch-data
  cargo run --release -p ygo-cli -- update-cards
fi

# Variables for the sessions (the setup script runs in its own shell, so they go to a file the sessions can source).
cat > .cloud-env <<ENV
export DUELFORGE_DATA="$PWD/data"
export VITE_MOCK=1
ENV
echo "ok: source .cloud-env for DUELFORGE_DATA and VITE_MOCK (unset VITE_MOCK for the real backend)"
