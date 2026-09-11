#!/usr/bin/env bash
set -euo pipefail

binary="$(realpath "$1")"
Xephyr :100 -screen 1280x1024 -listen tcp -nolisten local >/tmp/verge-xephyr.log 2>&1 &
xephyr=$!
trap 'kill "$xephyr" 2>/dev/null || true' EXIT

for _ in {1..50}; do
  DISPLAY=localhost:100 xset q >/dev/null 2>&1 && break
  sleep .1
done
DISPLAY=localhost:100 xset q >/dev/null 2>&1 || { cat /tmp/verge-xephyr.log; exit 1; }
DISPLAY=localhost:100 python3 "$(dirname "$0")/native_smoke.py" "$binary"
