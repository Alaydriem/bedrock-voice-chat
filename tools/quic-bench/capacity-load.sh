#!/usr/bin/env bash
# Runs INSIDE bvcload, which holds every client. Subcommands:
#   start <players> <speakers>      start the clients and print the stats window
#   collect <players> <end_ms>      wait for the stats files, stop the clients, print results
#
# The window is chosen on this container's clock. bvcsut shares the host kernel's
# clock, so `measure` there sees the same instant; the driver's machine does not.
set -uo pipefail

HOST=bvcsut
. /root/bench-lib.sh

# The first client creates the group (6 s), the rest join 0.4 s apart, then the
# room settles before counting starts.
BOOT_SETTLE_S=${BOOT_SETTLE_S:-10}

sum_client_cpu() {
  local p=$1 i total=0
  for i in $(seq 1 "$p"); do
    total=$((total + $(cpu_of "$(cat "$ROOT/bots/$i/pid")")))
  done
  echo "$total"
}

case "${1:-}" in
  start)
    p=$2
    s=$3
    gen_wav >/dev/null || exit 1
    start_ms=$(($(date +%s%3N) + (6 + BOOT_SETTLE_S) * 1000 + p * 400))
    end_ms=$((start_ms + WINDOW * 1000))
    rm -f "$ROOT/loadcpu"
    for i in $(seq 1 "$p"); do
      d="$ROOT/bots/$i"
      rm -f "$d/stats.json"
      extra=()
      [ "$i" -gt "$s" ] && extra=(--listen-only)
      "$BCAST" --certs-dir "$d" --server-name "$HOST:$PORT" \
        --audio-file "$WAV" --group --frame-ms "$FRAME_MS" \
        --stats "$d/stats.json" --stats-window "$start_ms" "$end_ms" "${extra[@]}" \
        > "$d/out.log" 2>&1 < /dev/null &
      echo $! > "$d/pid"
      [ "$i" = 1 ] && sleep 6 || sleep 0.4
    done
    (
      now_ms=$(date +%s%3N)
      [ "$now_ms" -lt "$start_ms" ] && sleep "$(awk -v d=$((start_ms - now_ms)) 'BEGIN { printf "%.3f", d / 1000 }')"
      c0=$(sum_client_cpu "$p")
      sleep "$WINDOW"
      c1=$(sum_client_cpu "$p")
      echo $(((c1 - c0) * 10000 / HZ / WINDOW)) > "$ROOT/loadcpu"
    ) > /dev/null 2>&1 < /dev/null &
    echo "WINDOW $start_ms $end_ms"
    ;;
  collect)
    p=$2
    end_ms=$3
    # Each client writes its file 1 s after end_ms; a client that died never does.
    deadline=$((end_ms + 15000))
    while :; do
      n=0
      for i in $(seq 1 "$p"); do [ -s "$ROOT/bots/$i/stats.json" ] && n=$((n + 1)); done
      [ "$n" -ge "$p" ] && [ -s "$ROOT/loadcpu" ] && break
      [ "$(date +%s%3N)" -ge "$deadline" ] && break
      sleep 1
    done
    echo "ALIVE $(count_alive "$p")"
    stop_clients "$p"
    echo "LOADCPU $(cat "$ROOT/loadcpu" 2>/dev/null || echo -1)"
    echo "CORES $(nproc)"
    for i in $(seq 1 "$p"); do
      [ -s "$ROOT/bots/$i/stats.json" ] && echo "STATS $i $(cat "$ROOT/bots/$i/stats.json")"
    done
    # A missing stats file is a verdict (INVALID), not a script failure.
    exit 0
    ;;
  *)
    echo "FATAL: usage: capacity-load.sh start|collect"
    exit 1
    ;;
esac
