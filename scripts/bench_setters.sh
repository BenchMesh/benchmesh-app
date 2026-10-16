#!/usr/bin/env bash
# Live setter check of the Rust device definitions (migration Phase 3).
# SAFE ONLY with nothing connected to any instrument output/input (user-confirmed 2026-10-16).
# Every block restores the state found before the test and verifies it by read-back.
set -uo pipefail
cd "$(dirname "$0")/.."
P="./target/debug/bm-probe --config ${CONFIG:-captures/bench-config.yaml}"
fail=0
get() { $P get --id "$1" "$2" "$3" ${4:+--ch $4} 2>&1; }
set_() { $P set --id "$1" "$2" "$3" "$4" ${5:+--ch $5} 2>&1; }
expect() { # id class op expected-substring [ch]
  local out; out=$(get "$1" "$2" "$3" "${5:-}")
  if [[ "$out" == *"$4"* ]]; then echo "  ok   $1 $2/$3 -> $out"; else echo "  FAIL $1 $2/$3 expected *$4* got $out"; fail=1; fi
}
refuse() { # id class op value expected-message-substring
  local out; out=$(set_ "$1" "$2" "$3" "$4")
  if [[ "$out" == *"$5"* ]]; then echo "  ok   refused $1 $2/$3=$4"; else echo "  FAIL $1 $2/$3=$4 not refused: $out"; fail=1; fi
}
cargo build -q -p bm-probe || exit 1

echo "== TENMA"
set_ tenma-72-2540 PSU voltage 5 >/dev/null;   expect tenma-72-2540 PSU voltage '"number":"5"'
set_ tenma-72-2540 PSU current 0.5 >/dev/null; expect tenma-72-2540 PSU current '"number":"500"'
set_ tenma-72-2540 PSU output ON >/dev/null;   sleep 1; expect tenma-72-2540 PSU output_voltage '"number":"5"'
set_ tenma-72-2540 PSU output OFF >/dev/null;  sleep 1; expect tenma-72-2540 PSU output_voltage '"number":"0"'
refuse tenma-72-2540 PSU voltage 61 "outside"
set_ tenma-72-2540 PSU voltage 4.94 >/dev/null; set_ tenma-72-2540 PSU current 0.988 >/dev/null
expect tenma-72-2540 PSU voltage '"number":"4.94"'; expect tenma-72-2540 PSU current '"number":"988"'

echo "== SPM PSU"
set_ owon-spm3103 PSU voltage 5 >/dev/null;  expect owon-spm3103 PSU voltage '"number":"5"'
set_ owon-spm3103 PSU output ON >/dev/null;  sleep 1.5; expect owon-spm3103 PSU output '"ON"'
set_ owon-spm3103 PSU output OFF >/dev/null; sleep 1; expect owon-spm3103 PSU output '"OFF"'
set_ owon-spm3103 PSU voltage 9 >/dev/null;  expect owon-spm3103 PSU voltage '"number":"9"'
echo "== SPM DMM"
set_ owon-spm3103 DMM mode VOLT_AC >/dev/null; expect owon-spm3103 DMM function '"VOLT_AC"'
set_ owon-spm3103 DMM range AUTO >/dev/null;   expect owon-spm3103 DMM function '"VOLT_AC"'
set_ owon-spm3103 DMM mode VOLT_DC >/dev/null; expect owon-spm3103 DMM function '"VOLT_DC"'
set_ owon-spm3103 DMM range AUTO >/dev/null

echo "== XDM"
set_ owon-xdm1241 DMM mode VOLT_AC >/dev/null; expect owon-xdm1241 DMM function '"VOLT_AC"'
set_ owon-xdm1241 DMM rate F >/dev/null;       expect owon-xdm1241 DMM rate '"F"'
set_ owon-xdm1241 DMM rate S >/dev/null;       expect owon-xdm1241 DMM rate '"S"'
set_ owon-xdm1241 DMM mode VOLT >/dev/null;    expect owon-xdm1241 DMM function '"VOLT"'

echo "== OEL"
refuse owon-oel1515 ELL volt 10 "MODE=VOLT"
set_ owon-oel1515 ELL mode VOLT >/dev/null; set_ owon-oel1515 ELL volt 10 >/dev/null; expect owon-oel1515 ELL volt '10'
set_ owon-oel1515 ELL volt 55 >/dev/null; expect owon-oel1515 ELL volt '55'
set_ owon-oel1515 ELL mode CURR >/dev/null; expect owon-oel1515 ELL curr '1'
set_ owon-oel1515 ELL input ON >/dev/null;  expect owon-oel1515 ELL input '"ON"'
set_ owon-oel1515 ELL input OFF >/dev/null; expect owon-oel1515 ELL input '"OFF"'

echo; [[ $fail == 0 ]] && echo "BENCH SETTERS PASS" || echo "BENCH SETTERS FAIL"
exit $fail
