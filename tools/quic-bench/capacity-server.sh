#!/usr/bin/env bash
# Runs INSIDE bvcsut, the server under test. Subcommands:
#   setup <players>             boot a fresh server and mint <players> identities
#   measure <start_ms> <window> wait for start_ms, then sample server CPU over the window
#   stop
set -uo pipefail

HOST=bvcsut
LISTEN=0.0.0.0
. /root/bench-lib.sh

stop_server() {
  [ -s "$SRV/pid" ] && kill "$(cat "$SRV/pid")" 2>/dev/null
  sleep 2
}

case "${1:-}" in
  setup)
    stop_server
    setup_server 0 "$2" || exit 1
    echo "READY"
    ;;
  measure)
    start_ms=$2
    window=$3
    spid=$(cat "$SRV/pid")
    now_ms=$(date +%s%3N)
    if [ "$now_ms" -lt "$start_ms" ]; then
      sleep "$(awk -v d=$((start_ms - now_ms)) 'BEGIN { printf "%.3f", d / 1000 }')"
    fi
    kill -0 "$spid" 2>/dev/null || { echo "FATAL: server not running at window start"; exit 1; }
    c0=$(cpu_of "$spid")
    sleep "$window"
    c1=$(cpu_of "$spid")
    kill -0 "$spid" 2>/dev/null || { echo "FATAL: server died during the window"; exit 1; }
    rss=$(awk '/^VmRSS:/{print $2}' "/proc/$spid/status")
    # grep -c prints "0" AND exits 1 on zero matches; head -1 keeps one line either way.
    pf=$( (grep -c "Failed to parse session packet" "$SRV/server.log" 2>/dev/null || echo 0) | head -1)
    echo "SRVCPU $(((c1 - c0) * 10000 / HZ / window)) RSS $rss PARSEFAIL $pf"
    ;;
  stop)
    stop_server
    echo "STOPPED"
    ;;
  *)
    echo "FATAL: usage: capacity-server.sh setup|measure|stop"
    exit 1
    ;;
esac
