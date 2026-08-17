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
| `BATCH_WAIT_MICROS` | unset | Writes a `voice { send_batch_wait_micros }` block. The key now exists and the server DEFAULTS to 7500 when the block is absent — pass `BATCH_WAIT_MICROS=0` explicitly to measure an unbatched arm; unset no longer means off |
| `MER_WORKERS` | unset | `MERS=1` only: writes `workers = N` into meridian's config. Unset uses meridian's default (cores − 2), which makes the io_uring backend's 1ms per-worker wakeup tick the dominant cost on a large host — pin it when comparing backends |

## Resolved: why the transmit path shows `sendmsg` — musl unrolls `sendmmsg`

`MODE=strace` reports `recvmmsg` on the receive side but plain `sendmsg` on the send
side, roughly 1:1 with delivered datagrams (0.84–0.99 across runs). That looked
internally inconsistent: `s2n-quic-platform`'s `io/tokio/task.rs` selects `rx` and
`tx` from the *same* branch, so a build using `recvmmsg` should also be using
`sendmmsg`, and `syscall/mmsg.rs::send` calls `libc::sendmmsg` unconditionally with
no single-message special case.

**Resolution (2026-08-17): the degradation is in musl, not s2n.** The bench server is
a musl-static binary, and musl's `src/network/sendmmsg.c` emulates `sendmmsg` as a
userspace loop of `sendmsg` calls on every 64-bit target:

```c
#if LONG_MAX > INT_MAX
	/* Can't use the syscall directly because the kernel has the wrong
	 * idea for the types of msg_iovlen, msg_controllen, and cmsg_len,
	 * and the cmsg blocks cannot be modified in-place. */
	...
	for (i=0; i<vlen; i++) {
		ssize_t r = sendmsg(fd, &msgvec[i].msg_hdr, flags);
```

`recvmmsg` is passed through as a real syscall (musl only zeroes the 64-bit pad
fields first), which is exactly the rx/tx asymmetry strace shows.

Corroborated on the artifacts rather than only on the sources. The glibc-linked
`broadcast` binary imports both wrappers, so s2n's mmsg path is genuinely compiled in
and genuinely reaches libc — the loss cannot be on the s2n side:

```console
$ readelf --dyn-syms -W bvc-bench-bins/broadcast.t2479 | grep -Ei 'sendm|recvm'
  UND sendmmsg@GLIBC_2.14
  UND recvmmsg@GLIBC_2.12
```

Note the binaries are stripped, so `objdump -T` reports nothing here; use
`readelf --dyn-syms`. The cfg situation was verified from the build-script output
(`bvc-target/x86_64-unknown-linux-musl/release/build/s2n-quic-platform-*/output`):
`socket_mmsg`, `gso`, `gro` and the rest are ALL already set for both the musl and
gnu builds, so `S2N_QUIC_PLATFORM_FEATURES_OVERRIDE` changes nothing here.

Consequences:

- On a musl build, cross-destination syscall batching via `sendmmsg` is
  unreachable no matter what s2n does — the loop is inside libc. Measuring a
  gnu-built server is the only way to see true `sendmmsg` batching.
- **GSO is unaffected.** It is a socket option plus a cmsg, the kernel does the
  segmentation, and musl passes `sendmsg` cmsgs through untouched — so
  same-destination bursts (which per-connection send batching produces) still
  collapse into fewer syscalls on musl.

### Answered 2026-08-17: glibc batches 18 datagrams per call, musl batches none

`MODE=strace`, 10p/10s, 15 s, same container spec, run back to back:

| | musl server | glibc server |
| --- | --- | --- |
| send syscall | `sendmsg` x 64,915 | `sendmmsg` x **3,696** |
| datagrams per call | 1.04 | **18.3** |
| time in send syscalls | 2.889 s | 0.210 s |
| total traced syscall time | 3.238 s | 0.451 s |

s2n **does** gather across destinations. Switching the deployment libc is worth a
17.6x reduction in send syscall count and 13.7x less time inside them, with no code
change and no added latency.

**This run does not apportion credit, because the binaries differ by more than libc.**
The musl arm was `bvc-server.t2479` (allocation and serialization work only); the glibc
arm was built later from a tree that also contains the per-connection `SendBatcher`. The
18.3 figure is therefore both effects compounding. What holds unconditionally: the musl
arm emits ~1 `sendmsg` per datagram, so on musl no amount of BVC-side batching reaches
the kernel as fewer syscalls.

To apportion it, build both libcs from one tree and compare — the harness supports it
directly via `BVC_SERVER_BIN`.

**Apportioned 2026-08-17, both libcs from one tree** (arm D + `SendBatcher`), wait pinned
via `BATCH_WAIT_MICROS` so the config default cannot drift between arms:

| Arm | send syscalls / 67,500 datagrams | datagrams per call | time in send syscalls | 10p/10s CPU |
| --- | --- | --- | --- | --- |
| musl, wait=0 | ~1 per datagram (libc unroll) | 1 | ~2.9 s | 28.10% |
| glibc, wait=0 | 16,513 `sendmmsg` | 4.1 | 1.63 s | 26.71% |
| musl, wait=5000 | 16,511 `sendmsg` | 4.1 | 0.90 s | 15.71% |
| glibc, wait=5000 | 9,416 `sendmmsg` | 7.2 | 0.80 s | — |

The split: the libc switch alone cuts send syscalls 4x at zero latency but moves CPU by
~1.4 points — barely outside noise — because each datagram still rides its own UDP
packet and the per-packet kernel path (UDP stack, loopback softirq, client ACK volume)
is untouched. The send wait cuts CPU 44% on either libc because it packs ~4 datagrams
into each QUIC packet, removing the packets themselves. Syscall count is the visible
symptom; packet count is the cost. The two compose: glibc + wait is cheapest of all.

### Previously open, kept for the reasoning

Whether s2n presents **more than one message per `sendmmsg` call** is still unknown, and
a musl binary cannot answer it: the unroll makes a batch of eight and a batch of one
look identical from outside. The `broadcast` client cannot answer it either — a client
sends to one peer, so even a perfect implementation would mostly show `vlen == 1`. Only
a server doing fan-out can.

The experiment, when someone is building anyway: drop
`--target x86_64-unknown-linux-musl` so the server links glibc, then run `MODE=strace`.
The bench container is Ubuntu, so a glibc binary runs there with no image change.

| Outcome | Meaning |
| --- | --- |
| `sendmmsg` count ≈ datagrams ÷ N, N > 1 | s2n batches across destinations and musl is discarding a real win at zero latency cost. Revisit the deployment libc. |
| `sendmmsg` count ≈ datagram count | s2n does not batch; only a deliberate send wait creates the burst, and syscall reduction costs latency on every libc. |

Until that runs, treat "batching is worth X" as unquantified on musl. What *is* measured:
roughly half the per-datagram cost is kernel time (51% at 10p/10s on 1.86), which bounds
the prize from above.

Worth knowing before touching this: **`sendmmsg` is not multicast.** Each `mmsghdr`
carries its own destination, so one call delivers N independently addressed
datagrams to N different peers. Recipient selection is unaffected — proximity and
channel filtering happen in `route_audio_frame` before anything is enqueued, and a
rejected recipient never has a datagram built at all. Batching changes the syscall
count, never the bytes on the wire or who receives them.

Which implementation compiles in is a build-script cfg, `s2n_quic_platform_socket_mmsg`,
detected by link-probing the `sendmmsg`/`recvmmsg` symbols (`features/socket_mmsg.rs`).
It can be forced:

```bash
S2N_QUIC_PLATFORM_FEATURES_OVERRIDE=cmsg,socket_msg,socket_mmsg,mtu_disc,gso,gro,pktinfo,tos
```

**List every feature, not just `socket_mmsg`.** The override path in `build.rs` does
`return Ok(())` immediately, *before* the `match env.target_os` block that would
otherwise add `mtu_disc`, `gso`, `gro`, `pktinfo` and `tos` on Linux. Passing
`socket_mmsg` alone therefore silently disables GSO, GRO, MTU discovery, pktinfo and
TOS, and the run reads as a regression caused by the wrong thing. Verified against
`s2n-quic-platform-0.86.0`.

Confirm the cfg actually took effect — the build script prints nothing useful, so
check the compiler invocation:

```bash
cargo build -v -p bedrock-voice-chat-server --release 2>&1 \
  | grep -o 's2n_quic_platform_socket_mmsg'
```

Then re-run `MODE=strace` and read the syscall table:

| Observation | Meaning |
| --- | --- |
| `sendmmsg` replaces `sendmsg`, count ≈ datagrams ÷ batch size | the lever works |
| `sendmmsg` appears but count ≈ datagram count | cfg is active; batches are one message each, so the endpoint is transmitting per wakeup and the problem is upstream of the syscall |
| still `sendmsg` | the override did not reach `s2n-quic-platform`; check it is exported for the whole build, not just the top crate |

The second outcome is the interesting one. `send_datagram` calls
`wakeup_handle.wakeup()`, which collapses to a no-op only while the endpoint has not
yet run — so whether the endpoint sees one datagram or ten is a scheduling race
between the fan-out task, ten per-connection output tasks and the endpoint task.
Measured, it sees one.

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
| 2026-08-17 | same | **s2n 1.86 re-baseline** (arm C + 1.86 pin), 3 runs | — | 9.25% | 27.81% mean (27.51–28.03) | strace: 56,497 sendmsg / 67,500 expected datagrams = 0.84 (sendmsg 86% of syscall time; recv already via recvmmsg). perf split: 15.65% user / 16.75% kernel, 51% kernel share. udp_err=0. |
| 2026-08-17 | same | **arm D** thinning (PlayerEnum on a 16/s heartbeat, `encoded_length` removed), wait=0, 2 runs | — | 10.35% | 28.10% mean (27.93–28.26) | CPU unchanged vs re-baseline: thinning shrinks bytes, not packet count. It is a bandwidth and packing win only. |
| 2026-08-17 | same | **arm D + send batch wait 2000µs**, 2 runs | — | 9.26% | 21.16% mean (19.36–22.95) | −24% vs re-baseline. |
| 2026-08-17 | same | **arm D + send batch wait 5000µs**, 2 runs | — | 7.90% | 15.71% mean (14.11–17.31) | −44% vs re-baseline. strace: 16,511 sendmsg / 67,500 = 0.24 (~4.1 datagrams per send). perf: 6.50% user / 13.30% kernel. |
| 2026-08-17 | same | **arm D + send batch wait 7000µs**, 2 runs | — | 8.08% | 14.18% mean (13.71–14.65) | −49%. The curve past 5ms: +2ms buys ~1.5 points. |
| 2026-08-17 | same | **arm D + send batch wait 7500µs**, 2 runs | — | 7.55% | 13.38% mean (13.36–13.40) | −52% vs re-baseline. **Shipped default.** |
| 2026-08-17 | same | **arm D + send batch wait 10000µs**, 2 runs | — | 5.00% | 12.55% mean (11.93–13.16) | −55%; not shipped — the wait is half a frame period. |
| 2026-08-17 | same | **arm D, glibc build**, wait=0, 2 runs | — | 9.35% | 26.71% mean (26.50–26.91) | sendmmsg 16,513 / 67,500 = 0.24 (vlen≈4.1) with NO wait: s2n batches across destinations once libc stops unrolling. CPU delta vs musl ≈ noise — per-packet kernel cost dominates, not syscall entries. gnu at wait=5000: 9,416 sendmmsg (0.14, ~7.2 per call). |
| 2026-08-17 | same | **arm D, glibc build**, wait=5000, 2 runs | — | 8.11% | 16.26% mean (16.01–16.50) | Statistically identical to musl at the same wait (15.71%). With batching on, libc is CPU-neutral at 10 connections; gnu's sendmmsg advantage should grow with connection count. |
| 2026-08-17 | same | **arm D via Meridian (tokio)**, wait=0, 2 runs | — | — | srv 26.54% + mer 25.67% = 52.2% | Unbatched, the relay costs as much as the server: 2 syscalls + a task spawn + an alloc per forwarded packet. |
| 2026-08-17 | same | **arm D via Meridian (tokio)**, wait=7500, 2 runs | — | — | srv 14.79% + mer 10.31% = 25.1% | Meridian −60% with ZERO meridian changes — it forwards 1:1, so the send batch's packet reduction passes straight through. The voice path halved. |

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
