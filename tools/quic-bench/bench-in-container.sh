#!/usr/bin/env bash
# Runs INSIDE an LXD container created by lxd_drive.py. Self-contained: reads no
# host paths and expects only /root/bin/{bvc-server,broadcast,ca.crt,ca.key}
# plus, for MERS=1, /root/bin/meridian.
#
# MODE selects what is measured:
#   cells  - CPU per cell across SHAPES x MERS (the baseline table)
#   strace - UDP syscall counts against datagrams delivered, one shape
#   perf   - userspace/kernel CPU split, one shape, plus best-effort perf record
set -uo pipefail

BINDIR=/root/bin
ROOT=/root/bench
SRV=$ROOT/server
MERDIR=$ROOT/meridian
WAV=$ROOT/tone.wav
PORT=21443
QPORT=21444
MPORT=21500
MAPIPORT=21501
HOST=localhost
BIN=$BINDIR/bvc-server
BCAST=$BINDIR/broadcast
MERBIN=$BINDIR/meridian
HZ=$(getconf CLK_TCK)

MODE=${MODE:-cells}
WINDOW=${WINDOW:-60}
FRAME_MS=${FRAME_MS:-20}
SHAPES=${SHAPES:-5 1;5 5;10 10}
MERS=${MERS:-0}
P=${P:-10}
S=${S:-10}
TRACE_SECS=${TRACE_SECS:-15}
BATCH_WAIT_MICROS=${BATCH_WAIT_MICROS:-}

# Fields 14 and 15 of /proc/<pid>/stat are utime and stime in clock ticks.
cpu_of() { awk '{print $14+$15}' "/proc/$1/stat" 2>/dev/null || echo 0; }
utime_of() { awk '{print $14}' "/proc/$1/stat" 2>/dev/null || echo 0; }
stime_of() { awk '{print $15}' "/proc/$1/stat" 2>/dev/null || echo 0; }

pct() { printf '%d.%02d%%' $(($1 / 100)) $(($1 % 100)); }

gen_wav() {
  mkdir -p "$ROOT"
  [ -s "$WAV" ] && return 0
  echo "generating source audio..."
  python3 - "$WAV" <<'PY'
import array, math, random, sys, wave
sr, dur = 48000, 450
random.seed(7)
w = wave.open(sys.argv[1], "wb")
w.setnchannels(1); w.setsampwidth(2); w.setframerate(sr)
a = array.array("h")
for n in range(sr * dur):
    t = n / sr
    on = 1.0 if (t % 2.5) < 1.5 else 0.0
    env = on * (0.35 + 0.25 * math.sin(2 * math.pi * 0.7 * t))
    a.append(int(max(-1.0, min(1.0, random.uniform(-1, 1) * env)) * 32767))
w.writeframes(a.tobytes()); w.close()
PY
  # A missing audio file does not stop `broadcast`: it stays connected and sends
  # nothing, so every client still counts as alive and only the CPU figure --
  # which is the result -- reveals that no traffic flowed.
  [ -s "$WAV" ] || { echo "FATAL: source audio was not generated at $WAV"; return 1; }
}

write_certs() {
  rm -rf "$SRV"
  mkdir -p "$SRV/certs" "$ROOT/bots"
  cp "$BINDIR/ca.crt" "$BINDIR/ca.key" "$SRV/certs/"
  openssl req -newkey rsa:2048 -nodes -keyout "$SRV/certs/server.key" \
    -out "$SRV/server.csr" -subj "/CN=$HOST" >/dev/null 2>&1
  openssl x509 -req -in "$SRV/server.csr" -CA "$SRV/certs/ca.crt" \
    -CAkey "$SRV/certs/ca.key" -CAcreateserial -out "$SRV/certs/server.crt" -days 2 \
    -extfile <(printf "subjectAltName=DNS:%s,IP:127.0.0.1\n" "$HOST") >/dev/null 2>&1
  [ -s "$SRV/certs/server.crt" ] || { echo "FATAL: server cert not issued"; return 1; }
}

write_config() {
  local mer=$1 mblock="" vblock=""
  if [ "$mer" = "1" ]; then
    mblock=$(cat <<MEOF

  advertised_quic_ports = [$MPORT]

  meridian {
    url         = "https://127.0.0.1:$MAPIPORT"
    api_key     = "loadtestkey"
    instance_id = 1
    name        = "load"
    host        = "$HOST"
    backend     = "127.0.0.1"
  }
MEOF
)
  fi
  if [ -n "$BATCH_WAIT_MICROS" ]; then
    vblock=$(cat <<VEOF

voice {
  send_batch_wait_micros = $BATCH_WAIT_MICROS
}
VEOF
)
  fi

  cat > "$SRV/config.hcl" <<EOF
server {
  listen    = "127.0.0.1"
  port      = $PORT
  quic_port = $QPORT
$mblock

  tls {
    certificate = "$SRV/certs/server.crt"
    key         = "$SRV/certs/server.key"
    certs_path  = "$SRV/certs"
  }

  features {
    telemetry = false
  }
}

database {
  scheme   = "sqlite3"
  database = "$SRV/bvc.sqlite3"
}
$vblock
EOF
}

boot_server() {
  ( cd "$SRV" && exec "$BIN" server -c "$SRV/config.hcl" ) > "$SRV/server.log" 2>&1 &
  echo $! > "$SRV/pid"

  local code=""
  for _ in $(seq 1 40); do
    sleep 1
    code=$(curl -sk -o /dev/null -w '%{http_code}' "https://127.0.0.1:$PORT/health/readiness" 2>/dev/null)
    [ "$code" = "200" ] && break
    kill -0 "$(cat "$SRV/pid")" 2>/dev/null || {
      echo "FATAL: server exited during boot"
      tail -20 "$SRV/server.log"
      return 1
    }
  done
  [ "$code" = "200" ] || { echo "FATAL: server never became ready"; tail -20 "$SRV/server.log"; return 1; }
}

mint_identities() {
  local n=$1 ok=0 i out code resp d
  for i in $(seq 1 "$n"); do
    out=$("$BIN" admin generate-code -c "$SRV/config.hcl" -p "LoadBot$i" -g minecraft -d 3600 2>&1)
    code=$(printf '%s\n' "$out" | awk '/^Code: /{print $2; exit}')
    [ -z "$code" ] && { echo "mint failed for LoadBot$i"; continue; }
    resp=$(curl -sk -X POST "https://127.0.0.1:$PORT/api/auth/code" \
      -H 'Content-Type: application/json' -d "{\"code\":\"$code\"}")
    d="$ROOT/bots/$i"
    mkdir -p "$d"
    printf '%s' "$resp" | D="$d" python3 -c '
import json, os, sys
d = os.environ["D"]
body = json.load(sys.stdin)
n = body.get("data", body)
open(d + "/test.crt", "w").write(n["certificate"])
open(d + "/test.key", "w").write(n["certificate_key"])
open(d + "/ca.crt", "w").write(n["certificate_ca"])
' && ok=$((ok + 1)) || echo "redeem failed for LoadBot$i"
  done
  [ "$ok" -eq "$n" ] || { echo "FATAL: minted $ok/$n identities"; return 1; }
  echo "minted $n identities"
}

setup_server() {
  local mer=$1 n=$2
  write_certs || return 1
  write_config "$mer"
  boot_server || return 1
  mint_identities "$n" || return 1
}

start_meridian() {
  rm -rf "$MERDIR"
  mkdir -p "$MERDIR"
  cat > "$MERDIR/config.hcl" <<EOF
listen            = "127.0.0.1:$MPORT"
cid_prefix_length = 2

api {
  listen  = "127.0.0.1:$MAPIPORT"
  api_key = "loadtestkey"

  tls {
    certificate = "$SRV/certs/server.crt"
    key         = "$SRV/certs/server.key"
  }
}

backend "load" {
  hostname    = "$HOST"
  tcp_addr    = "127.0.0.1:$PORT"
  udp_addr    = "127.0.0.1:$QPORT"
  instance_id = 1
}
EOF
  ( cd "$MERDIR" && exec "$MERBIN" -c "$MERDIR/config.hcl" serve ) > "$MERDIR/mer.log" 2>&1 &
  echo $! > "$MERDIR/pid"
  sleep 4
  kill -0 "$(cat "$MERDIR/pid")" 2>/dev/null || {
    echo "FATAL: meridian exited during boot"
    tail -20 "$MERDIR/mer.log"
    return 1
  }
}

# Speakers are clients 1..s; the remainder connect and only receive.
start_clients() {
  local p=$1 s=$2 frame=$3
  shift 3
  local i extra
  for i in $(seq 1 "$p"); do
    extra=()
    [ "$i" -gt "$s" ] && extra=(--listen-only)
    "$BCAST" --certs-dir "$ROOT/bots/$i" --server-name "$HOST:$PORT" \
      --audio-file "$WAV" --group --frame-ms "$frame" "$@" "${extra[@]}" \
      > "$ROOT/bots/$i/out.log" 2>&1 &
    echo $! > "$ROOT/bots/$i/pid"
    # The first client pays for the group create; the rest only join it.
    [ "$i" = 1 ] && sleep 6 || sleep 0.4
  done
}

count_alive() {
  local p=$1 i up=0
  for i in $(seq 1 "$p"); do
    kill -0 "$(cat "$ROOT/bots/$i/pid")" 2>/dev/null && up=$((up + 1))
  done
  echo "$up"
}

report_first_failure() {
  local p=$1 i
  for i in $(seq 1 "$p"); do
    kill -0 "$(cat "$ROOT/bots/$i/pid")" 2>/dev/null || {
      echo "   first failure (client $i):"
      sed -n '1,10p' "$ROOT/bots/$i/out.log" | sed 's/^/     /'
      return
    }
  done
}

stop_clients() {
  local p=$1 i
  for i in $(seq 1 "$p"); do kill "$(cat "$ROOT/bots/$i/pid")" 2>/dev/null; done
}

stop_all() {
  local p=$1
  stop_clients "$p"
  [ -s "$MERDIR/pid" ] && kill "$(cat "$MERDIR/pid")" 2>/dev/null
  [ -s "$SRV/pid" ] && kill "$(cat "$SRV/pid")" 2>/dev/null
  sleep 3
}

cell() {
  local frame=$1 mer=$2 p=$3 s=$4

  setup_server "$mer" "$p" || return 1
  local spid mpid=""
  spid=$(cat "$SRV/pid")

  local dial=()
  if [ "$mer" = "1" ]; then
    start_meridian || { kill "$spid"; return 1; }
    mpid=$(cat "$MERDIR/pid")
    dial=(--quic-port "$MPORT")
  fi

  start_clients "$p" "$s" "$frame" "${dial[@]}"
  sleep 20

  local up
  up=$(count_alive "$p")

  local s0 s1 m0=0 m1=0
  s0=$(cpu_of "$spid")
  [ -n "$mpid" ] && m0=$(cpu_of "$mpid")
  sleep "$WINDOW"
  s1=$(cpu_of "$spid")
  [ -n "$mpid" ] && m1=$(cpu_of "$mpid")

  local pps=$((1000 / frame)) dg spct mpct tot rss
  dg=$((s * pps * p))
  spct=$(((s1 - s0) * 10000 / HZ / WINDOW))
  mpct=$(((m1 - m0) * 10000 / HZ / WINDOW))
  tot=$((spct + mpct))
  rss=$(awk '/^VmRSS:/{print $2}' "/proc/$spid/status" 2>/dev/null)

  printf '%-14s %-8s up=%2d/%2d dgram/s=%5d srv=%7s mer=%7s TOTAL=%7s rss=%3dMiB\n' \
    "${frame}ms $([ "$mer" = 1 ] && echo via-mer || echo direct)" "${p}p/${s}s" \
    "$up" "$p" "$dg" "$(pct "$spct")" "$(pct "$mpct")" "$(pct "$tot")" "$((rss / 1024))"

  [ "$up" -lt "$p" ] && report_first_failure "$p"

  # dgram/s above is arithmetic from the shape, not a measurement, so a run that
  # routes nothing still prints a plausible-looking row. Two checks stand between
  # that and a reported result.

  # The client shares the wire format with the server. A `broadcast` binary built
  # before a packet-layout change decodes nothing, and the server then does almost
  # no work -- which reads as an enormous CPU win rather than as a broken run.
  local parse_failures
  parse_failures=$(grep -c "Failed to parse session packet" "$SRV/server.log" 2>/dev/null || echo 0)
  if [ "$parse_failures" -gt 0 ]; then
    echo "   INVALID: server failed to parse $parse_failures inbound packets."
    echo "            The load client is a different wire version than the server."
    echo "            Rebuild both from the same tree; this row means nothing."
  fi

  # A generic floor for everything else that silently stops routing. The cost model
  # is ~1.6% fixed plus ~50us per datagram/s, so 40% of that is well below any
  # believable optimization and still catches a collapse.
  #
  # In hundredths of a percent, to match spct: one datagram/s at 50us is 5e-5 of a
  # core, which is half of one hundredth of a percent.
  local expected floor
  expected=$((160 + dg / 2))
  floor=$((expected * 40 / 100))
  if [ "$dg" -gt 0 ] && [ "$spct" -lt "$floor" ]; then
    echo "   SUSPECT: $(pct "$spct") is under 40% of the $(pct "$expected") this shape"
    echo "            costs. Verify delivery with MODE=strace before believing it."
  fi

  stop_all "$p"
}

mode_cells() {
  echo "=========== LXD container, ${FRAME_MS}ms frames ==========="
  local shape m
  IFS=';' read -ra SHAPE_LIST <<< "$SHAPES"
  for shape in "${SHAPE_LIST[@]}"; do
    set -- $shape
    for m in $MERS; do
      cell "$FRAME_MS" "$m" "$1" "$2"
    done
    echo
  done
}

mode_strace() {
  command -v strace >/dev/null 2>&1 || {
    echo "installing strace..."
    apt-get install -y -qq strace >/dev/null 2>&1 || { echo "FATAL: no strace"; return 1; }
  }

  setup_server 0 "$P" || return 1
  local spid
  spid=$(cat "$SRV/pid")
  start_clients "$P" "$S" "$FRAME_MS"
  sleep 18
  echo "clients alive: $(count_alive "$P")/$P"

  local pps=$((1000 / FRAME_MS)) expect_out expect_in
  expect_out=$((S * pps * (P - 1) * TRACE_SECS))
  expect_in=$((S * pps * TRACE_SECS))
  echo
  echo "expected over ${TRACE_SECS}s: $expect_out outbound datagrams, $expect_in inbound"
  echo "tracing pid $spid for ${TRACE_SECS}s ..."
  echo

  timeout --signal=INT "$TRACE_SECS" \
    strace -c -f -e trace=sendmsg,sendmmsg,sendto,recvmsg,recvmmsg,recvfrom \
    -p "$spid" 2>&1 | tail -25

  echo
  echo "=== ratio ==="
  echo "sendmsg / outbound datagrams near 1.00 means one UDP send per datagram (no coalescing)."
  echo "divide the traced sendmsg count by $expect_out to get it."

  stop_all "$P"
}

mode_perf() {
  setup_server 0 "$P" || return 1
  local spid
  spid=$(cat "$SRV/pid")
  start_clients "$P" "$S" "$FRAME_MS"
  sleep 18
  echo "clients alive: $(count_alive "$P")/$P"
  echo

  local u0 s0 u1 s1
  u0=$(utime_of "$spid")
  s0=$(stime_of "$spid")

  # perf is best effort: the container image's linux-tools rarely match the
  # host kernel, and the split below is what the measurement actually needs.
  local perf_ok=0
  if ! command -v perf >/dev/null 2>&1; then
    apt-get install -y -qq linux-tools-generic linux-tools-common >/dev/null 2>&1
    for c in /usr/lib/linux-tools/*/perf /usr/lib/linux-tools-*/perf; do
      [ -x "$c" ] && { ln -sf "$c" /usr/local/bin/perf; break; }
    done
  fi
  if command -v perf >/dev/null 2>&1; then
    echo "perf_event_paranoid=$(cat /proc/sys/kernel/perf_event_paranoid 2>/dev/null || echo '?')"
    ( cd "$ROOT" && timeout $((WINDOW - 5)) perf record -F 499 -g -p "$spid" \
      -o "$ROOT/perf.data" >/dev/null 2>"$ROOT/perf.err" ) &
    local perf_pid=$!
    perf_ok=1
  else
    echo "perf: not installable in this container"
  fi

  sleep "$WINDOW"
  u1=$(utime_of "$spid")
  s1=$(stime_of "$spid")
  [ "$perf_ok" = 1 ] && wait "$perf_pid" 2>/dev/null

  local ut st tot
  ut=$(((u1 - u0) * 10000 / HZ / WINDOW))
  st=$(((s1 - s0) * 10000 / HZ / WINDOW))
  tot=$((ut + st))
  echo
  echo "================= kernel vs userspace (${P}p/${S}s) ================="
  printf 'user   (userspace) : %s of one core\n' "$(pct "$ut")"
  printf 'system (kernel)    : %s of one core\n' "$(pct "$st")"
  printf 'total              : %s of one core\n' "$(pct "$tot")"
  [ "$tot" -gt 0 ] && printf 'kernel share       : %d%%\n' $((st * 100 / tot))
  echo "===================================================================="
  echo

  if [ "$perf_ok" = 1 ] && [ -s "$ROOT/perf.data" ]; then
    echo "=== perf report (top symbols, self time) ==="
    perf report -i "$ROOT/perf.data" --stdio --no-children --percent-limit 0.4 2>/dev/null | head -45
  else
    echo "no perf.data; perf stderr:"
    head -10 "$ROOT/perf.err" 2>/dev/null
  fi

  stop_all "$P"
}

echo "host: $(nproc) cores visible, $(awk '/MemTotal/{printf "%.1f", $2/1048576}' /proc/meminfo) GiB"
echo "kernel: $(uname -r)"
echo "mode: $MODE"
echo

gen_wav || exit 1

case "$MODE" in
  cells) mode_cells ;;
  strace) mode_strace ;;
  perf) mode_perf ;;
  *) echo "FATAL: unknown MODE '$MODE' (cells|strace|perf)"; exit 1 ;;
esac

echo "DONE"
