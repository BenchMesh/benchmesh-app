#!/usr/bin/env bash
# Run while the BenchMesh window is frozen: records which process/thread is stuck and what it waits
# on, so a JS loop (web process main thread at ~100 % CPU), a blocked wait (0 % CPU) and a stuck
# app process can be told apart. Writes ~/benchmesh-freeze-<time>.txt.
# With sudo it also takes gdb backtraces of every thread (ptrace is restricted otherwise):
#   sudo -E bash scripts/ui-freeze-capture.sh
set -u
out="${HOME}/benchmesh-freeze-$(date +%Y%m%d-%H%M%S).txt"
# process names: the app, its webviews (one per window) and WebKit's network process
pids=$(pgrep -d ' ' -x 'benchmesh|WebKitWebProces|WebKitNetworkPr' || true)
[ -n "$pids" ] || { echo 'BenchMesh is not running'; exit 1; }
{
  echo "== $(date)  uptime: $(uptime)"
  echo "== processes"
  ps -o pid,ppid,etime,%cpu,rss,stat,wchan:32,args -p ${pids// /,} 2>/dev/null
  echo "== CPU per thread over 3 s (top)"
  top -b -H -n 2 -d 3 -p ${pids// /,} 2>/dev/null | awk '/^top -/{n++} n==2' | head -60
  for p in $pids; do
    echo "== threads of $p ($(cat /proc/$p/comm 2>/dev/null)): state, kernel wait"
    for t in /proc/$p/task/*; do
      printf '%s %-18s %s %s\n' "${t##*/}" "$(cat $t/comm 2>/dev/null)" "$(awk '{print $3}' $t/stat 2>/dev/null)" "$(cat $t/wchan 2>/dev/null)"
    done
  done
  echo "== X windows of BenchMesh (to match the journal lines below)"
  xwininfo -root -tree 2>/dev/null | grep -i '("benchmesh"' || echo "no X windows (native Wayland?)"
  echo "== compositor: frames it never confirmed (GTK then stops drawing; frame_sync.rs)"
  journalctl --since '-2 days' --no-pager 2>/dev/null | grep 'Frame has assigned frame counter' | tail -10
  echo "== API answering? (server side)"
  curl -s -m 3 -o /dev/null -w 'GET /version -> %{http_code} in %{time_total}s\n' http://127.0.0.1:57666/version || echo "no answer"
  if [ "$(id -u)" = 0 ] && command -v gdb >/dev/null; then
    for p in $pids; do
      echo "== gdb backtraces of $p ($(cat /proc/$p/comm 2>/dev/null))"
      timeout 60 gdb -p "$p" -batch -ex 'set pagination off' -ex 'thread apply all bt 25' 2>/dev/null | grep -v '^\[' | head -400
    done
  fi
} > "$out" 2>&1
echo "written $out"
