---
title: Upgrading to 1.0.0-beta.21
description: Voice protocol 3.0.0, the peering rewrite, and every configuration key that changed since 1.0.0-beta.20.
sidebar:
  label: 1.0.0-beta.21
  order: 2
---

Bedrock Voice Chat 1.0.0-beta.21 changes the voice protocol, replaces the peering system, and moves several configuration keys.

## Upgrade steps

1. Back up the database.
2. Stop the server.
3. Add `addon_mode` to every entry in `server.bedrock.servers`. Use `"net"` or `"no_net"`. The server does not start without it.
4. Delete the `server.bedrock.dns` and `relay` blocks from `config.hcl`. Both are ignored in this release, not rejected.
5. Rename `log.out` to `log.path` in `config.hcl`. The server does not start while `out` is present.
6. Replace the server binary, or pull the new container image.
7. Start the server. Migrations run at startup.
8. Read the startup log before letting players connect.
9. Update the Bedrock Addon to 1.0.0-beta.21.
10. Publish the 1.0.0-beta.21 client. Older clients cannot connect.

:::danger
Step 1 is not optional. Step 6 runs the `drop_peer_link_permission` migration, which deletes every `peer_link` grant. Its rollback restores nothing. Which players held the grant is recorded nowhere else.
:::

Three deployments carry extra steps.

| Also running | Extra step |
|---|---|
| Java mod in embedded mode | Rewrite `embedded-config` as nested blocks before step 5. See [Java mod](#java-mod). |
| Peering | Relink every pair with `bvc relay peerlink` after step 6. A 1.0.0-beta.20 peer link does not carry over. See [peering](#peering). |
| Hytale Java mod | Remove it. It is no longer built. |

No other existing key in `config.hcl` has to change.

## Voice protocol 3.0.0

The voice protocol moves from `2.1.0` to `3.0.0`. Clients on 1.0.0-beta.20 and earlier cannot open a session against a 1.0.0-beta.21 server, and a 1.0.0-beta.21 client cannot open one against an older server. Only the major and minor are compared.

No configuration controls this. Ship the server and the client together.

## Configuration changes

File: `config.hcl`

| Key | Change | Action |
|---|---|---|
| `server.bedrock.servers[].addon_mode` | New. **Required** on every entry. | Add `addon_mode = "net"` or `addon_mode = "no_net"` to each entry. |
| `server.bedrock.dns` | Removed. | Delete the block. |
| `relay` | Removed. Replaced by `server.peers` and `server.peer_relay_url`. | Delete the block. Redeclare peers. |
| `voice.recording` | New. `enabled` defaults to `true`. | Set `enabled = false` to forbid client-side recording. |
| `voice.limits` | New. `connections` defaults to `0`, which admits everyone. | Nothing. Set `connections` to cap concurrent voice sessions. |
| `log.out` | Removed. Replaced by `log.path`. | Rename the key. See [Logging](#logging). |
| `log.path` | New. Directory for the log file. Defaults to `./logs`. | Nothing, unless you want the file elsewhere. |

`addon_mode` has no default. A `servers` list carrying an entry without it does not parse, and the server does not start. Every key is listed in the [configuration reference](/wiki/reference/configuration/).

:::caution
Every other removed key is **ignored, not rejected**. A leftover `relay` or `server.bedrock.dns` block starts the server with those settings absent and writes nothing to the log.
:::

`voice.recording.enabled` takes the `BVC_RECORDING` environment override.

`voice.limits` needs no action on upgrade. The default admits everyone, so a server that does not declare the block behaves exactly as it did. `connections` and `reconnect_grace` take the `BVC_MAX_CONNECTIONS` and `BVC_RECONNECT_GRACE` overrides.

## Logging

The server now writes to two places at once. Neither is selectable.

- **The console**, on **stderr**, human-readable and coloured.
- **A file**, in `log.path`, one JSON object per line.

### `log.out` is replaced by `log.path`

`log.out` chose *where everything went*: `"stdout"` sent every line to standard output and nothing to a file, and any other value sent every line to a file and nothing to the console. Console and file are now independent and both always on, which that key cannot express.

A `config.hcl` that still names `out` **fails to start** with an error naming the key. This is the one removed key in this release that is rejected rather than ignored, because silently dropping it would put the log somewhere other than where you asked.

```hcl
log {
  level = "info"
  path  = "./logs"
}
```

`log.path` is a directory, created if missing. If it cannot be created or written, the server logs one line to stderr and starts anyway, console-only. It does not refuse to start.

### Console output moved from stdout to stderr

Standard output now carries only the output of CLI subcommands. Update any supervisor unit, container configuration, or shell pipeline that read the server log from stdout. Docker, systemd, and the Java mod capture both streams and need no change.

Colour is suppressed automatically when stderr is not a terminal, so a redirected log no longer collects escape sequences. `NO_COLOR=1` forces it off and `CLICOLOR_FORCE=1` forces it on.

### The file is JSON, and rotates by size

Every line is one object. `instance_id` and `name` come from `server.meridian` and are `null` when that block is absent; `boot_id` is new per process start, so a restart is visible inside one file.

```json
{"ts":"2026-08-23T18:43:32.989Z","level":"info","target":"bvc_server_lib::runtime","msg":"Protocol Version: 3.0.0","instance_id":null,"name":null,"boot_id":"01a02fef-63fc-7ab3-80f0-76853ffb4bf2","file":"server/src/runtime/mod.rs","line":164}
```

Fields added at a call site appear as top-level keys, so `jq '.bind'` reads them directly.

Rotation is now by **size, not by day**:

| Before | Now |
|---|---|
| `bvc-server.log.2026-08-23` | `bvc-server.log` plus `bvc-server_<timestamp>.log` archives |
| One file per day, kept forever | 50 MB per file, ten archives kept |

Update any logrotate rule or log shipper pattern that matches the old names.

### `RUST_LOG`

`RUST_LOG` overrides `log.level` and the built-in per-crate defaults in one string, and governs both the server's own output and its dependencies'.

```
RUST_LOG=info,bvc_server_lib::stream=debug,hyper=off
```

A directive with no `=` sets the global level. A directive with `=` sets the level for every target whose name starts with the text on the left; the longest matching prefix wins. `off` silences a target completely. An unparseable directive is named on stderr and skipped.

Only this `target=level` form is supported. The span and field syntax some Rust tools accept (`target[span{field=value}]=level`) is not.

## Scripts that call the API

Two request shapes changed. A script of your own that calls either has to change with it. The Addon, the Java mods, and the client ship with the change.

| Route | Change |
|---|---|
| `/api/position`, `/api/control`, `/api/state`, `/api/preferences`, `/api/audio/event`, the chat WebSocket | Send `Authorization: Bearer <token>` instead of `X-MC-Access-Token: <token>`. The token value is unchanged. |
| `POST /api/bedrock/transfer` | Drop the `xuid` field. The body is `host` and `port`. |

## QUIC ports

`server.advertised_quic_ports` publishes the UDP ports a client tries, in preference order. A non-empty list **replaces** `server.quic_port` as the set clients attempt. It does not add to it. An empty list leaves `quic_port` as the only published port.

File: `config.hcl`

```hcl
server {
    quic_port             = 443
    advertised_quic_ports = [443, 8443]
}
```

Use this where a proxy fronts the server on a port other than the bind port. The server binds one socket regardless of how many ports are published.

The environment override is `BVC_ADVERTISED_QUIC_PORTS`.

## WebSocket voice transport

Voice now also runs over WebSocket on the HTTPS port. It shares `server.port` (443 by default) and needs no firewall change. The server advertises it on `/api/config` unconditionally, and there is no key that turns it off.

A client tries the published QUIC ports first and falls back to WebSocket when none answer.

### Forcing WebSocket-only voice

Make QUIC unreachable. There is no dedicated switch.

File: `config.hcl`

```hcl
server {
    quic_port             = 0
    advertised_quic_ports = [1]
}
```

`quic_port = 0` binds the QUIC listener to a port the operating system assigns. `advertised_quic_ports = [1]` publishes a port nothing listens on. Every QUIC attempt fails and voice runs over WebSocket on 443.

:::note
This costs every client the full QUIC probe before it falls back. Sessions start slower.
:::

## Peering

:::caution
Peering is currently experimental, and is subject to change. Do not build anything on the peering protocol until a more formal definition is announced.
:::

Peering is rewritten. A 1.0.0-beta.20 peer link does not carry over.

| 1.0.0-beta.20 | 1.0.0-beta.21 |
|---|---|
| `relay` config block | `server.peers` blocks and `server.peer_relay_url` |
| `POST /api/relay/offer` | Removed |
| `POST /api/relay/peer_link` | `GET /api/admin/relay/peerlink`, or `bvc relay peerlink` |
| `POST /api/relay/peer_redeem` | Removed |
| `peer_link` player permission | Removed. The declaration in `config.hcl` is the authorization |
| `server::{host}:{port}` certificates | Refused at connect |

Relink each pair of servers. Run `bvc relay peerlink` on each side and paste the printed `peers` block into the other's `config.hcl`. See [peering](/wiki/reference/peering/).

Capabilities default to `carry_speakers` alone. `query_audio` and `serve_audio` are granted only where named. An explicitly empty `capabilities` list stays empty.

Where two peers have no direct path, set `server.peer_relay_url`.

:::note
`server.peer_relay_url` is removed after beta.21. Peers connect directly or not at all, and there is no relay to point at. See [peering](/wiki/reference/peering/).
:::

## Java mod

`embedded-config` now mirrors the BVC server's own configuration. The flat keys are gone.

:::caution
The embedded server **refuses to start** while any old key remains. It logs each one with its replacement and stops. The mod does not rewrite the file.
:::

File: `plugins/BedrockVoiceChat/config.yml` (Paper), `config/bedrock-voice-chat.json` (Fabric)

| Old key | New key |
|---|---|
| `http-port` | `server.port` |
| `quic-port` | `server.quic_port` |
| `tls-certificate` | `server.tls.certificate` |
| `tls-key` | `server.tls.key` |
| `tls-names` | `server.tls.names` |
| `tls-ips` | `server.tls.ips` |
| `assets-path` | `server.assets_path` |
| `broadcast-range` | `voice.spatial_audio.broadcast_range` |
| `log-level` | `log.level` |
| `allow-audio-upload` | `permissions.defaults.audio_upload` |
| `allow-audio-delete` | `permissions.defaults.audio_delete` |

The camelCase spellings (`httpPort`, `tlsCertificate`, and the rest) map the same way.

Rewritten as nested blocks:

```yaml
use-embedded-server: true
embedded-config:
  server:
    port: 8444
    quic_port: 8443
    tls:
      certificate: "/path/to/fullchain.pem"
      key: "/path/to/privkey.pem"
      names:
        - "bvc.example.com"
  voice:
    spatial_audio:
      broadcast_range: 32.0
  log:
    level: "info"
```

Anything left out takes the server's default. See the [Java mod](/wiki/server/java-mod/) pages.

## Bedrock Addon

Each entry in `server.bedrock.servers` carries an `addon_mode` of `net` or `no_net`, declaring who delivers position, state, and chat events for that world.

The compact environment form gains a trailing token:

```
Name@host[:port][@protocol][@net|@no_net]
```

The two trailing tokens are read by shape. Either may be omitted, and their order does not matter.

## Hytale

The Hytale Java mod is removed, along with its build and release pipeline.
