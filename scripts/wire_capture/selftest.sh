#!/usr/bin/env bash
# End-to-end self-test of the wire capture tooling against a simulated TENMA on a pty pair.
# Starts: socat pty pair, simulator, traced service (port 57991). Cleans up everything on exit.
set -euo pipefail

HERE="$(cd "$(dirname "$0")" && pwd)"
PY="${PYTHON:-python3}"
PORT="${SELFTEST_PORT:-57991}"
TMP="$(mktemp -d)"
PIDS=()

cleanup() {
  for p in "${PIDS[@]}"; do kill "$p" 2>/dev/null || true; done
  for p in "${PIDS[@]}"; do wait "$p" 2>/dev/null || true; done
  rm -rf "$TMP"
}
trap cleanup EXIT INT TERM

socat pty,raw,echo=0,link="$TMP/tty_dev" pty,raw,echo=0,link="$TMP/tty_sim" 2>/dev/null &
PIDS+=($!)
for _ in $(seq 50); do [[ -e "$TMP/tty_dev" && -e "$TMP/tty_sim" ]] && break; sleep 0.1; done

"$PY" "$HERE/sim_tenma.py" "$TMP/tty_sim" &
PIDS+=($!)

cat > "$TMP/config.yaml" <<EOF
version: 1
devices:
  - id: sim-tenma
    name: Simulated Tenma
    driver: tenma_72
    model: 72-2540
    transport: serial
    port: $TMP/tty_dev
    baud: 9600
    serial: 8N1
EOF

"$PY" "$HERE/run_traced_service.py" --config "$TMP/config.yaml" --out "$TMP/captures" --port "$PORT" --force --sim-pty &
PIDS+=($!)
# socat holds the pty, hence --force (the in-use guard is for real instruments)

for _ in $(seq 100); do [[ -f "$TMP/captures/LATEST" ]] && break; sleep 0.1; done
[[ -f "$TMP/captures/LATEST" ]] || { echo "FAIL: traced service did not start"; exit 1; }

"$PY" "$HERE/exercise.py" --session "$(cat "$TMP/captures/LATEST")" \
  --passive "${SELFTEST_PASSIVE:-10}" --rounds 1

SESSION="$(cat "$TMP/captures/LATEST")"
if [[ -n "${KEEP_SELFTEST:-}" ]]; then trap 'cp -r "$SESSION" "$KEEP_SELFTEST" 2>/dev/null; cleanup' EXIT; fi
"$PY" "$HERE/summarize.py" "$SESSION"

# Assertions: the capture must contain the protocol essentials
grep -q '"text":"\*IDN?"' "$SESSION/sim-tenma.jsonl"          || { echo "FAIL: no *IDN? captured"; exit 1; }
grep -q '"text":"VOUT1?"' "$SESSION/sim-tenma.jsonl"          || { echo "FAIL: no VOUT1? captured"; exit 1; }
grep -q '"line":"DTR","value":false' "$SESSION/sim-tenma.jsonl" \
                                                               || { echo "FAIL: DTR low not captured"; exit 1; }
grep -q 'STATUS?' "$SESSION/sim-tenma.transcript.txt"          || { echo "FAIL: transcript missing STATUS?"; exit 1; }
grep -q '"phase": "passive"' "$SESSION/api.jsonl"              || { echo "FAIL: no registry snapshots"; exit 1; }
grep -q '"phase": "query"' "$SESSION/api.jsonl"                || { echo "FAIL: no query calls"; exit 1; }

echo "SELFTEST PASS"
