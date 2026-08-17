# quic-bench

Measures BVC server CPU per delivered voice datagram against real QUIC clients.

A throwaway LXD container is created over the LXD HTTPS API, the prebuilt
binaries and one benchmark script are pushed into it, the script runs, and the
container is deleted. Clients and server are co-resident, so **all traffic is
loopback** — no NIC, driver or interrupt handling is exercised. Numbers are
comparable to each other, not to a deployment behind a real network.

## Requirements

- An LXD host reachable over its HTTPS API.
- A trusted LXD client certificate pair in `$LXD_CERTS`, named
  `lxd-client.crt` and `lxd-client.key`.
- `python3` on the driving machine. No third-party packages.

## Build the binaries first

The server is built as musl so it is static and needs no runtime libraries in
the container. `broadcast` is built natively — it links ALSA through `cpal`,
which does not cross-compile, and the linker drops ALSA as unused so the
resulting binary needs only base glibc.

```bash
cd server && CARGO_TARGET_DIR=$HOME/bvc-target \
  cargo build -p bedrock-voice-chat-server --release \
  --target x86_64-unknown-linux-musl

cd server && CARGO_TARGET_DIR=$HOME/bvc-target \
  cargo build -p bedrock-voice-chat-server --release --example broadcast
```

**Build both, every time.** `broadcast` shares the wire format with the server. A
client binary built before a packet-layout change decodes nothing, the server
does almost no work, and the run reports an enormous CPU improvement instead of
a failure. The harness now counts `Failed to parse session packet` in the server
log and marks such a row `INVALID`, but rebuilding both is the actual fix.

Meridian is only needed for `MERS=1`:

```bash
cd ../meridian && CARGO_TARGET_DIR=$HOME/mer-target cargo build --release
```

## Run

The baseline table — three shapes, direct path, no Meridian:

```bash
LXD_CERTS=../bvc/tools/swarm LXD_HOST=https://10.57.2.3:8443 \
SHAPES="5 1;5 5;10 10" MERS="0" WINDOW=60 \
  python3 tools/quic-bench/lxd_drive.py
```

One line per cell:

```
20ms direct     10p/10s  up=10/10 dgram/s= 5000 srv= 31.05% mer=  0.00% TOTAL= 31.05% rss= 36MiB
```

`dgram/s` is the delivered rate, `speakers x (1000/frame_ms) x players`. `srv`
and `mer` are percentages of one core, from `utime + stime` in
`/proc/<pid>/stat` over `WINDOW` seconds.

### Syscall counts

Whether `s2n-quic` coalesces queued datagrams into single UDP sends:

```bash
MODE=strace P=10 S=10 TRACE_SECS=15 CT_PRIVILEGED=1 \
LXD_CERTS=../bvc/tools/swarm python3 tools/quic-bench/lxd_drive.py
```

Divide the reported `sendmsg` count by the printed expected outbound datagram
count. A ratio near 1.00 means one UDP send per datagram.

### Userspace versus kernel split

Which half of the cost an optimization can even reach:

```bash
MODE=perf P=10 S=10 WINDOW=40 CT_PRIVILEGED=1 \
LXD_CERTS=../bvc/tools/swarm python3 tools/quic-bench/lxd_drive.py
```

`perf record` is attempted but usually fails: the container image's
`linux-tools` packages must match the *host* kernel, and a 24.04 image against
a 5.15 host has no matching build. The `utime`/`stime` split does not depend on
it and always reports.

## Environment

| Variable | Default | Meaning |
| --- | --- | --- |
| `LXD_HOST` | `https://10.57.2.3:8443` | LXD API endpoint |
| `LXD_CERTS` | `~/.config/bvc-quic-bench` | Directory holding `lxd-client.{crt,key}` |
| `BVC_CA` | `server/server/certificates` | Directory holding the `ca.crt` / `ca.key` the server signs player certs with |
| `BVC_TARGET_DIR` | `~/bvc-target` | Cargo target dir the two BVC binaries were built into |
| `BVC_SERVER_BIN` | `$BVC_TARGET_DIR/x86_64-unknown-linux-musl/release/bvc-server` | Server binary |
| `BVC_BROADCAST_BIN` | `$BVC_TARGET_DIR/release/examples/broadcast` | Load client |
| `MERIDIAN_BIN` | `~/mer-target/release/meridian` | Only read when `MERS` includes `1` |
| `CT_NAME` | `bvcbench` | Container name; deleted and recreated per run |
| `CT_CORES` | `8` | `limits.cpu` |
| `CT_MEM` | `6GB` | `limits.memory` |
| `CT_IMAGE` | `24.04` | Ubuntu release alias |
| `CT_PRIVILEGED` | unset | Set to any value for `strace` and `perf` modes |
| `MODE` | `cells` | `cells`, `strace` or `perf` |
| `SHAPES` | `5 1;5 5;10 10` | `cells` only: `players speakers` pairs, `;`-separated |
| `MERS` | `0` | `cells` only: `0`, `1`, or `0 1` for both paths |
| `WINDOW` | `60` | Measurement window in seconds |
| `FRAME_MS` | `20` | Opus frame duration |
| `P` / `S` | `10` / `10` | `strace` and `perf` only: players and speakers |
| `TRACE_SECS` | `15` | `strace` only |
| `BATCH_WAIT_MICROS` | unset | Writes a `voice { send_batch_wait_micros }` block; leave unset unless that key exists in the build under test |

## Reading the numbers

**Variance is large.** The host runs other workloads and the container is not
pinned. The same 10p/10s shape has read 31.05% and 37.67% twenty minutes apart
— roughly ±10%. A change under about 2 percentage points at 10p/10s is not
distinguishable from noise on a single run. Re-measure the baseline immediately
before and after a change rather than comparing against a figure from hours
earlier, and repeat any result you intend to act on.

Every cell restarts the server and mints fresh identities, so a cell cannot
inherit state from the one before it.

## Baseline

Recorded here as each measurement run completes, so later runs compare against
a figure from this harness rather than against prose elsewhere.

| Date | Container | Build | 5p/1s | 5p/5s | 10p/10s | Notes |
| --- | --- | --- | --- | --- | --- | --- |
| 2026-08-17 | 8 core / 6 GB, ubuntu 24.04, host kernel 5.15.0-139 | pre-optimization, ad-hoc scripts | 3.03% | 10.46% | 31.05% | 20 ms frames, direct path |
| 2026-08-17 | same | same | — | — | 37.67% | `MODE=perf`, 40 s window: 11.67% user / 26.00% kernel, 69% kernel share |
| 2026-08-17 | same | same, **this harness** | 3.08% | 9.78% | 32.08% | Reference baseline. Reproduces the ad-hoc numbers within 7%. |
| 2026-08-17 | same | **arm A** pre-optimization, 4 runs | — | 10.38% mean | 31.30% mean (30.11–32.71) | Reference arm. |
| 2026-08-17 | same | **arm B** serialize-once fan-out, 4 runs | — | 9.60% mean | 27.70% mean (26.90–28.30) | −7.5% / −11.5% against A. |
| 2026-08-17 | same | **arm C** B plus allocation removal, 2 runs | — | 9.45% mean | 27.34% mean (26.51–28.18) | −1.3% against B: inside noise. |

### Cost model

Five shapes, both arms, two rounds each, 20 ms frames, direct path. `dgram/s` is
`speakers x 50 x players`. The 60%-speaking shapes are the realistic ones: a room
with working mic gating does not have everybody transmitting at once.

| Shape | dgram/s | arm A baseline | arm C optimized | Change |
| --- | --- | --- | --- | --- |
| 5p/3s | 750 | 6.46% | 6.24% | −3.5% |
| 5p/5s | 1,250 | 10.38% | 9.45% | −8.9% |
| 10p/6s | 3,000 | 19.79% | 18.13% | −8.4% |
| 10p/10s | 5,000 | 31.30% | 27.34% | −12.6% |
| 15p/9s | 6,750 | 38.98% | 34.27% | −12.1% |

Least squares over those five points:

```
baseline   = 3.22% + 54.2 us per datagram/sec
optimized  = 3.47% + 46.6 us per datagram/sec
```

The marginal cost per delivered datagram fell **14%**. The saving grows with fan-out
breadth because it comes from serializing once per frame instead of once per
recipient, which is why 5p/3s barely moves and 15p/9s moves most.

**`udp_err` was 0 at every point in both arms** — no `SndbufErrors`, no
`RcvbufErrors`, no `InErrors` over any 60 s window. Default socket buffers are
dropping nothing at these rates.

All three arms were built against s2n-quic 1.81. The pin moved to 1.86 later the same day, so
re-measure A and C before comparing anything built after that.

Reference binaries are kept in `~/bvc-bench-bins/` rather than inside the cargo target directory,
because `cargo sweep` walks target dirs and a swept reference arm cannot be reconstructed without
rebuilding an older tree. Point `BVC_SERVER_BIN` and `BVC_BROADCAST_BIN` at a matched pair from
there; they must always be changed together.

Run-to-run spread on one binary at 10p/10s has been 29.6–32.1% over a 60 s
window, so single runs are not comparable. Alternate the two binaries in one
sitting and compare the means; that is what the rows above do.
