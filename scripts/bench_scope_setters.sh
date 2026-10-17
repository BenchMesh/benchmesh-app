#!/usr/bin/env bash
# Live check of every RIGOL DHO800/DHO900 setter through the device definition:
# set a value, read it back, restore the original. Configuration only (nothing energises).
#
# usage: scripts/bench_scope_setters.sh [config] [device-id]
#   default config: captures/rigol-config.yaml, device: rigol-dho804
set -u
cd "$(dirname "$0")/.."
CONFIG=${1:-captures/rigol-config.yaml}
ID=${2:-rigol-dho804}
cargo build -q -p bm-probe || exit 1
probe() { ./target/debug/bm-probe --config "$CONFIG" "$@" 2>&1 | tail -1; }

pass=0; fail=0
check() { # op value [ch]
  local op=$1 val=$2 ch=${3:-1} before after
  before=$(probe get --id "$ID" OSC "$op" --ch "$ch")
  probe set --id "$ID" OSC "$op" "$val" --ch "$ch" >/dev/null
  after=$(probe get --id "$ID" OSC "$op" --ch "$ch")
  probe set --id "$ID" OSC "$op" "$(restore_value "$before")" --ch "$ch" >/dev/null
  if [ "$after" != "$before" ]; then
    pass=$((pass + 1)); echo "ok   $op ch$ch: $before -> $val -> $after (restored)"
  else
    fail=$((fail + 1)); echo "FAIL $op ch$ch: $before -> $val -> $after (no change)"
  fi
}
# `get` prints {"value": V}; V is a string, a number or an SI dict (use its `val`).
restore_value() {
  python3 -c 'import json,sys; v=json.loads(sys.argv[1])["value"]; v=v["val"] if isinstance(v,dict) else v; print(int(v) if isinstance(v,float) and v.is_integer() else v)' "$1"
}

check display ON 3
check scale 0.1
check offset 0.02
check coupling AC
check probe 10
check bwlimit 20M
check invert ON
check timebase 1e-5
check timebase_offset 1e-6
check trigger_level 0.01
check trigger_slope NEG
check trigger_source CHAN2
check trigger_sweep NORM
check acquire_type PEAK
echo "$pass passed, $fail failed"
[ "$fail" -eq 0 ]
