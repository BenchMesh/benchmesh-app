#!/usr/bin/env bash
# Linux: does a BenchMesh build keep painting when the window manager never confirms a frame?
# That is what GNOME 42 under XWayland sometimes does (journal: "Frame has assigned frame counter
# but no frame drawn time"), and GTK 3 with frame synchronisation then stops drawing for good
# (crates/benchmesh-app/src/frame_sync.rs). Runs the app on the simulated bench in a nested X
# server (Xephyr, a window on your desktop for about 40 s) under fakewm.c, which announces
# _NET_WM_FRAME_DRAWN and never sends it, starts a recording so the recorder timer ticks, and
# compares three captures of the window. Local check only (needs a display, gcc, libX11, Xephyr,
# ImageMagick); not for CI.
#   scripts/frame-sync/check.sh [app executable]     # default target/debug/benchmesh
set -u
repo=$(cd "$(dirname "$0")/../.." && pwd)
app=${1:-$repo/target/debug/benchmesh}
work=$(mktemp -d)
trap 'kill -TERM $AP 2>/dev/null; sleep 2; kill -KILL $AP 2>/dev/null; kill $FW $XP 2>/dev/null; wait 2>/dev/null; rm -rf "$work"' EXIT
gcc -O1 -o "$work/fakewm" "$(dirname "$0")/fakewm.c" -lX11 || exit 1
D=:97
Xephyr $D -screen 1400x900 -ac -br -noreset >/dev/null 2>&1 & XP=$!
sleep 2
DISPLAY=$D "$work/fakewm" >/dev/null & FW=$!
sleep 1
env -u WAYLAND_DISPLAY DISPLAY=$D GDK_BACKEND=x11 BENCHMESH_DATA_DIR="$work/data" BENCHMESH_ONLINE=off \
  BENCHMESH_CONFIG="$repo/captures/bench-config.yaml" BENCHMESH_SIMULATE="$repo/captures/20261016-203514" \
  BENCHMESH_UI="$repo/frontend/dist" BENCHMESH_DEVICES="$repo/devices" "$app" >"$work/app.log" 2>&1 & AP=$!
sleep 20
url=$(grep -o 'http://127.0.0.1:[0-9]*' "$work/app.log" | head -1)
[ -n "$url" ] || { echo "the app did not start:"; cat "$work/app.log"; exit 1; }
curl -s -m 3 -X POST -H 'Content-Type: application/json' -d '{"name":"tick","interval_s":1}' "$url/recordings" >/dev/null
sleep 5
win=$(DISPLAY=$D xwininfo -root -tree | grep '"BenchMesh"' | awk '{print $1}' | head -1)
for i in 1 2 3; do DISPLAY=$D xwd -silent -id "$win" | convert xwd:- "$work/$i.png"; sleep 3; done
a=$(compare -metric AE "$work/1.png" "$work/2.png" null: 2>&1)
b=$(compare -metric AE "$work/2.png" "$work/3.png" null: 2>&1)
if [ "$a" = 0 ] && [ "$b" = 0 ]; then echo "FROZEN: the window did not change in 6 s"; exit 1; fi
echo "OK: the window keeps painting (changed pixels: $a, $b)"
