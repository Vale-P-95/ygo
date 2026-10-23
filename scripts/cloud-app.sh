#!/usr/bin/env bash
# The real Tauri app (WebKitGTK) without a window, under Xvfb, with a screenshot.
# usage: scripts/cloud-app.sh <abs-out.png> [wait-seconds]   (needs scripts/cloud-setup.sh; VITE_MOCK=1 for the mock backend)
# The first debug build takes ~8 minutes: start it with run_in_background.
set -euo pipefail
cd "$(dirname "$0")/../apps/desktop"
out=$(realpath -m "${1:-$OLDPWD/out/xvfb/app.png}"); wait_s=${2:-20}
mkdir -p "$(dirname "$out")"
: "${DUELFORGE_DATA:=$PWD/../../data}"; export DUELFORGE_DATA
Xvfb :98 -screen 0 1280x800x24 -nolisten tcp & xvfb=$!
trap 'pkill -P $$ || true; kill $xvfb 2>/dev/null || true' EXIT
DISPLAY=:98 pnpm tauri dev >"$(dirname "$out")/log.txt" 2>&1 &
# The window appears once the build ends; poll for the binary, then let the page load.
until pgrep -f target/debug/duelforge-desktop >/dev/null; do sleep 5; done
sleep "$wait_s"
DISPLAY=:98 import -window root "$out"
echo "screenshot: $out"
