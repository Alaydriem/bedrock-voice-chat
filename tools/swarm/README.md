# swarm — distributed BVC voice load harness (LXD)

Drives many **real** headless BVC clients (`bvc_client_e2e` — full DSP → Opus →
QUIC → jitter buffer) inside ephemeral **LXD containers** on your home Linux
servers, against one BVC server, then reports per-container audio delivery and
the server's own routing metrics. Everything is driven by one config file.

## Topology

```
  ┌─ this machine (Windows dev box) ───────────────┐        ┌─ home-a (Linux + LXD) ─┐
  │  BVC server under test  +  swarm controller     │  LAN   │  container ×N (bots)   │
  │  (talks LXD REST API to each home host)  ───────┼───────▶│  container ×N (bots)   │
  └─────────────────────────────────────────────────┘        └────────────────────────┘
                                                    └───────▶ home-b (Linux + LXD) ...
```

The controller and BVC server run on the same already-set-up machine; the bot
load runs on separate LXD hosts so it never steals CPU from the server under
test. LXD is Linux-only, so this Windows controller talks to the daemons over
their HTTPS REST API with a client cert — no `lxc` client needed here.

---

## Validated run procedure (this environment, 2026-07-20)

The exact, working steps for THIS repo/network. Follow top to bottom.

### 0. Environment facts (already set up)
- **Controller + BVC server:** this Windows dev box (Ryzen 9950X, 32T).
- **Server DNS:** `local.bedrockvc.stream:443` — resolvable network-wide (bots in
  containers reach it by name; TLS cert is public, trusted via webpki roots).
- **LXD hosts:** `balminuel` = `10.57.2.3`, `vaya` = `10.57.2.6`, API on `:8443`.
  The client cert `tools/swarm/lxd-client.crt/.key` is **already `lxc config trust`-added**
  on both.
- **Container image:** `26.04` (pulled from cloud-images simplestreams);
  `cloud-init.yaml` installs webkit2gtk-4.1 / gtk-3-0t64 / libasound2t64 / xvfb.
  Confirmed working headless — bots run under `xvfb-run` with no early-exit.

### 1. Build the binaries (once, or after code changes)
The Linux artifacts are built in **WSL** (cargo at `~/.cargo/bin`), which shares this
checkout and writes ELF binaries into the same `target/` dirs:
```
# controller (Windows) — standalone workspace
cargo build --release --manifest-path tools/swarm/Cargo.toml   # -> tools/swarm/target/release/swarm.exe

# Linux bot + Linux swarm (in WSL)
wsl bash -lc 'cd /mnt/c/Users/charl/projects/bvc && cargo build -p bvc-client-e2e --release'   # -> target/release/bvc_client_e2e (ELF)
wsl bash -lc 'cd /mnt/c/Users/charl/projects/bvc/tools/swarm && cargo build --release'         # -> tools/swarm/target/release/swarm (ELF)

# BVC server (release, for real capacity numbers — debug is only ~20% slower here)
cd server/server && cargo build --release --bin bvc-server                                      # -> server/target/release/bvc-server.exe
```

### 2. Start the BVC server — FROM `server/server/` (CWD gotcha!)
The DB path is `sqlite://./bvc.sqlite3`, **relative to CWD**. Run from the repo
root and it silently creates an *empty* DB (no players → minting 403s). Always:
```
cd server/server && ../target/release/bvc-server.exe server -c config.hcl
# verify: curl -k https://local.bedrockvc.stream:443/api/config  → {"status":"Ok",...}
```

### 3. Extract an admin identity for minting (once)
Minting hits `/api/admin/*`, which needs an mTLS client cert whose player holds
the `admin` permission. Pull one straight from the DB:
```
sqlite3 server/server/bvc.sqlite3 \
  "SELECT p.id,p.gamertag FROM player p JOIN player_permission pp ON pp.player_id=p.id \
   WHERE pp.permission='admin' AND pp.effect=1;"          # -> e.g. 8|Alaydriem
sqlite3 server/server/bvc.sqlite3 "SELECT certificate     FROM player WHERE id=8;" > tools/swarm/certs/admin.crt
sqlite3 server/server/bvc.sqlite3 "SELECT certificate_key FROM player WHERE id=8;" > tools/swarm/certs/admin.key
```
Verify (Windows curl can't present PEM client certs — Schannel — so use WSL curl,
which uses OpenSSL):
```
wsl bash -c 'curl -sk --cert tools/swarm/certs/admin.crt --key tools/swarm/certs/admin.key \
  -w " HTTP %{http_code}\n" https://local.bedrockvc.stream:443/api/admin/permission/minecraft/Alaydriem'
# expect HTTP 200 with an "admin":"allow" entry
```

### 4. The working `swarm.toml` (at repo root)
```toml
server = "https://local.bedrockvc.stream:443"
ca = "C:/Users/charl/projects/bvc/certificates/ca.crt"
admin_cert = "C:/Users/charl/projects/bvc/tools/swarm/certs/admin.crt"
admin_key  = "C:/Users/charl/projects/bvc/tools/swarm/certs/admin.key"
access_token = "<server minecraft.access_token from server/server/config.hcl>"
prefix = "SwarmBot"
group_size = 5
duration_secs = 45
client_bin = "C:/Users/charl/projects/bvc/target/release/bvc_client_e2e"
swarm_bin  = "C:/Users/charl/projects/bvc/tools/swarm/target/release/swarm"

[lxd]
client_cert = "C:/Users/charl/projects/bvc/tools/swarm/lxd-client.crt"
client_key  = "C:/Users/charl/projects/bvc/tools/swarm/lxd-client.key"
image = "26.04"
cloud_init = "C:/Users/charl/projects/bvc/tools/swarm/cloud-init.yaml"

[[target]]
name = "balminuel"
endpoint = "https://10.57.2.3:8443"
containers = 1
bots_per_container = 5

[[target]]
name = "vaya"
endpoint = "https://10.57.2.6:8443"
containers = 1
bots_per_container = 5
```

### 5. Run
```
./tools/swarm/target/release/swarm.exe controller --config swarm.toml
```
First launch per image pulls 26.04 + runs cloud-init (~1–2 min); subsequent runs
still re-run cloud-init because containers are ephemeral (bake a base image if you
want faster spin-up).

### 6. Ramp to find capacity
Edit `swarm.toml` between runs. Deepening the mesh (`group_size = bots_per_container`)
stresses the server hardest per bot (fan-out = server work). Sequence used here:
`10/mesh10 → 60/mesh30 → 100/mesh50`. At 100 bots × mesh-50 **both LXD hosts CPU-peg**
(that's the client-side ceiling of a 2-host generator, not the server).

### 7. Instrument the SERVER while a run streams (real CPU/bandwidth)
`route_audio_frame` duration alone misses QUIC encrypt+`sendmsg` (~7/8 of server
CPU). To get true numbers, run `tools/swarm/measure-server.ps1` in another shell
while a run is streaming — it waits for streaming, samples `bvc-server` process CPU
+ NIC egress + the `/metrics` rate over 30 s, and prints per-delivery constants.

### 8. Recovering from an interrupted run (orphaned containers)
If you Ctrl-C / kill the controller mid-run, its teardown is skipped and ephemeral
containers keep running. List and stop them (stop auto-deletes ephemerals):
```
C=tools/swarm
for h in 10.57.2.3 10.57.2.6; do
  for n in $(wsl bash -c "curl -sk --cert $C/lxd-client.crt --key $C/lxd-client.key https://$h:8443/1.0/instances" | grep -oE 'swarm-[a-z0-9-]+'); do
    wsl bash -c "curl -sk --cert $C/lxd-client.crt --key $C/lxd-client.key -X PUT https://$h:8443/1.0/instances/$n/state -d '{\"action\":\"stop\",\"force\":true}'"
  done
done
```

---

## What a run does

`swarm controller --config swarm.toml`:

1. Mints a bot player + single-use login code per bot via the admin API (or
   reuses a `--codes` file).
2. Scrapes the server's `/metrics` (baseline).
3. On each target, launches `containers` ephemeral containers concurrently;
   for each: waits for cloud-init, pushes the `swarm` + `bvc_client_e2e`
   binaries and the job, execs the in-container agent (under `xvfb-run`), and
   collects its report. Ephemeral containers auto-delete on stop.
4. Scrapes `/metrics` again and prints per-container + total delivery and the
   server-side routing delta over the run window.

## Prerequisites

**Controller machine** (runs the BVC server + `swarm controller`):
- `swarm` built for this machine. It is a **standalone workspace** (deliberately
  not part of the root workspace), so build it from its own directory:
  `cd tools/swarm && cargo build --release` (or
  `cargo build --release --manifest-path tools/swarm/Cargo.toml`).
- Admin mTLS cert/key + server CA (paths in the config).
- An LXD **client** identity (`certs/lxd-client.crt` + `.key`).
- Prebuilt **Linux** `bvc_client_e2e` and `swarm` artifacts (see config
  `client_bin` / `swarm_bin`) to push into containers.

**Each home LXD host:**
- LXD installed and its HTTPS API reachable (`lxc config set core.https_address :8443`).
- The controller's client cert trusted: `lxc config trust add certs/lxd-client.crt`.
- Outbound access to the image server (or the image pre-cached).
- The container runtime deps come from `cloud-init.yaml`; no manual install.

## LXD client cert (one-time)

Generate a client identity and trust it on each host:

```
# on the controller
openssl req -x509 -newkey rsa:2048 -nodes -days 3650 \
  -keyout certs/lxd-client.key -out certs/lxd-client.crt -subj "/CN=bvc-swarm"

# on each home host (copy the .crt over first)
lxc config trust add lxd-client.crt
lxc config set core.https_address :8443
```

Optionally pin each daemon's server cert (`server_cert` in the target); omit it
to accept the daemon's self-signed cert on the LAN.

## Configuration

Copy `swarm.example.toml` to `swarm.toml` and edit it — every field is
documented inline. Essentials:

| field | meaning |
|-------|---------|
| `server` | BVC server base URL (this machine's LAN address) |
| `ca` | server CA PEM path; embedded into container jobs |
| `admin_cert` / `admin_key` | admin mTLS identity, minting only |
| `access_token` | server mod token, sent as `X-MC-Access-Token` for positions |
| `client_bin` / `swarm_bin` | prebuilt **Linux** artifacts pushed into containers |
| `group_size` | bots per voice group (each group shares a channel) |
| `duration_secs` | seconds each bot streams |
| `[lxd]` | `client_cert`/`client_key`, `image`, `cloud_init` |
| `[[target]]` | one per LXD host: `endpoint`, optional `server_cert`, `containers`, `bots_per_container` |

Total bots = sum over targets of `containers × bots_per_container`. Per-host
grouping is `containers = 1`; one voice group per container is
`bots_per_container = group_size`.

## Running

```
# On a Linux box / CI: build the artifacts and copy them to artifacts/linux/
cargo build -p bvc-client-e2e --release                       # -> bvc_client_e2e (root workspace)
cargo build --release --manifest-path tools/swarm/Cargo.toml  # -> swarm (standalone workspace)

# On the controller (this machine): start the BVC server, then
./target/release/swarm controller --config swarm.toml
```

Split minting from running (reuse codes across runs):

```
swarm mint --config swarm.toml --out codes.txt
swarm controller --config swarm.toml --codes codes.txt
```

## Reading the output

```
==================== SWARM RESULTS ====================
container home-a#0                  10/10 connected  sent=  60000  recv=  240000
container home-a#1                  10/10 connected  sent=  60000  recv=  240000
container home-b#0                  10/10 connected  sent=  60000  recv=  240000
-------------------------------------------------------
TOTAL 30/30 connected  sent=180000  recv=720000

server-side routing over the run window:
  frames routed:       900000
  recipient drops:     0
  mean route duration: 78.4 µs
=======================================================
```

- **connected** below total → bots failed to connect (codes/certs/container deps).
- **recipient drops > 0** → the server's per-recipient output queues overflowed;
  the first user-audible failure mode. Its onset as you ramp is the capacity ceiling.
- **recv vs sent**: within a group of G bots each sending S frames, every bot
  should receive ~(G−1)·S; a large shortfall is delivery loss.
- **mean route duration** rising sharply under load is the server CPU signal.

## Ramp protocol (finding the breaking point)

Hold the 50-bot shape first (`group_size = 5`, 0 drops expected), then push:
raise `group_size` (10 → 25 → 50; 50 = full fan-out), then raise total bots
(add containers/targets: 75 → 100 → …) until `recipient drops` climbs or bots
stop connecting. That curve is the empirical per-CPU capacity of the server box.

## Measured capacity (release server, 2026-07-20)

Server: Ryzen 9950X / Windows. Constants derived from an instrumented run at
100 bots × mesh-50 (see step 7):

| Constant | Value | Notes |
|----------|-------|-------|
| CPU per delivery | **13.0 µs** | dominated by QUIC encrypt + `sendmsg`; release ≈ debug |
| Throughput per core | **~77k deliveries/s** (~54k at 70% headroom) | linear, per-connection-parallel |
| Wire bytes per delivery | **266 B** | OS/build-independent egress |
| Memory | ~40 MB base + **~0.3 MB/connection** | 100 conns = 72 MB; never binds |
| Drops through 100 bots/mesh-50 | **0** | server sat at ~1/32 cores; the LXD hosts pegged first |

Caveats: measured on **Windows** — Linux (DO/Hetzner) with GSO/`sendmmsg` is
typically 2–4× cheaper per delivery, so these are conservative floors. Load model:
`deliveries/s = clients × talk_ratio × 50 fps × audible_peers`. At a proximity
profile (20% talking, 8 audible peers = 80 deliveries/s, 170 kbps per client), even
a 2-core VPS carries ~1,300+ clients; past ~6,000 concurrent you become **1 GbE
NIC-bound**, not CPU-bound, and monthly-transfer cost dominates.

## Notes

- Login codes + access token travel in the agent **job pushed as a file**, and
  the admin cert/key never leave the controller.
- `xvfb` **is** needed — the exec wraps the agent in `xvfb-run -a`; confirmed
  working on 26.04 (bots run headless, no early exit).
- `cloud-init.yaml` package names target the t64 transition (24.04/26.04):
  `libwebkit2gtk-4.1-0`, `libgtk-3-0t64`, `libasound2t64`. Adjust for older images.
- **Windows curl uses Schannel** and cannot present a PEM client cert; use WSL
  curl (OpenSSL) for any manual mTLS probe of LXD or the admin API. The `swarm`
  controller itself uses reqwest+rustls, so it does mTLS fine on Windows.
- The server DB is `sqlite://./bvc.sqlite3` **relative to CWD** — always start the
  server from `server/server/`, or it creates an empty DB and minting 403s.
