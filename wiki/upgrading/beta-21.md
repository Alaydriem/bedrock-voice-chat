---
title: "Upgrading to 1.0.0-beta.21"
description: "Voice protocol 3.0.0, the peering rewrite, and every configuration key that changed since 1.0.0-beta.20."
source: https://www.bedrockvoicechat.com/wiki/upgrading/beta-21/
---
Bedrock Voice Chat 1.0.0-beta.21 changes the voice protocol, replaces the peering system, removes the Bedrock transfer relay, and moves several configuration keys.

## Upgrade steps

1. Back up the database.
2. Stop the server.
3. Add `addon_mode` to every entry in `server.bedrock.servers`. Use `"net"` or `"no_net"`. See [Bedrock Addon](#bedrock-addon) for the value to use. The server does not start without it.
4. Delete the `server.bedrock.dns` and `relay` blocks from `config.hcl`. Delete the `enabled`, `transfer_port`, `transfer_target_port` and `transfer_cache_ttl_secs` keys from `server.bedrock`. The server ignores these keys and shows no error for them.
5. Rename `log.out` to `log.path` in `config.hcl`. The server does not start while `out` is present.
6. Replace the server binary, or pull the new container image.
7. Start the server. Migrations run at startup.
8. Read the startup log before you let players connect.
9. Update the Bedrock Addon to 1.0.0-beta.21.
10. Tell your players to update the client to 1.0.0-beta.21. Older clients cannot connect.

:::danger
Do not skip step 1. Step 7 runs the `drop_peer_link_permission` migration, which deletes every `peer_link` grant. Its rollback restores nothing. No other record shows which players had the grant. Only your backup has it.
:::

Three deployment types need extra steps.

| Also running | Extra step |
|---|---|
| Java mod in embedded mode | Rewrite `embedded-config` as nested blocks before step 6. See [Java mod](#java-mod). |
| Peering | Link every pair of servers again with `bvc relay peerlink` after step 7. A 1.0.0-beta.20 peer link does not work in 1.0.0-beta.21. See [peering](#peering). |
| Hytale Java mod | Remove it. It is no longer built. |

No other key in `config.hcl` has to change.

## Voice protocol 3.0.0

The voice protocol changes from `2.1.0` to `3.0.0`. Clients on 1.0.0-beta.20 and earlier cannot open a session on a 1.0.0-beta.21 server. A 1.0.0-beta.21 client cannot open a session on an older server. Only the major and minor version numbers are compared.

No configuration key changes this. Update the server and the client together.

## Configuration changes

File: `config.hcl`

| Key | Change | Action |
|---|---|---|
| `server.bedrock.servers[].addon_mode` | New. **Required** on every entry. | Add `addon_mode = "net"` or `addon_mode = "no_net"` to each entry. |
| `server.bedrock.dns` | Removed. | Delete the block. |
| `server.bedrock.enabled` | Removed. Bedrock support is always on. | Delete the key. |
| `server.bedrock.transfer_port` | Removed with the transfer relay. | Delete the key. Close 28283/udp. |
| `server.bedrock.transfer_target_port` | Removed with the transfer relay. | Delete the key. |
| `server.bedrock.transfer_cache_ttl_secs` | Removed with the transfer relay. | Delete the key. |
| `relay` | Removed. Replaced by `server.peers` and `server.peer_relay_url`. | Delete the block. Declare the peers again. |
| `voice.recording` | New. `enabled` defaults to `true`. | Set `enabled = false` to stop clients from recording. |
| `voice.limits` | New. `connections` defaults to `0`. `0` sets no limit. | None. Set `connections` to limit the number of voice sessions. |
| `log.out` | Removed. Replaced by `log.path`. | Rename the key. See [Logging](#logging). |
| `log.path` | New. Directory for the log file. Defaults to `./logs`. | None, unless you want the file in a different directory. |

`addon_mode` has no default. If an entry in `servers` has no `addon_mode`, the server cannot read the configuration and does not start. All keys are in the [configuration reference](/wiki/reference/configuration/).

:::caution
The server ignores every other removed key. With a leftover `relay` block, `server.bedrock.dns` block, or `server.bedrock` transfer key, the server starts without those settings and writes nothing to the log.
:::

The `BVC_RECORDING` environment variable overrides `voice.recording.enabled`.

`voice.limits` needs no action. With no `voice.limits` block, the server has no connection limit, the same as 1.0.0-beta.20. The `BVC_MAX_CONNECTIONS` and `BVC_RECONNECT_GRACE` environment variables override `connections` and `reconnect_grace`. See [`voice.limits`](/wiki/reference/configuration/#voicelimits).

## Logging

The server now always writes to two outputs. You cannot turn off either output.

- **The console**, on **stderr**: human-readable text with colour.
- **A file**, in `log.path`: one JSON object per line.

### `log.path`

Old behaviour: `log.out = "stdout"` sent all output to standard output and nothing to a file. Any other value sent all output to a file and nothing to the console. Now the server always writes to both.

If `config.hcl` still contains `out`, the server **does not start**. The error names the key. `log.out` is the only removed key that stops the server.

```hcl
log {
    level = "info"
    path  = "./logs"
}
```

`log.path` is a directory. The server creates it if it does not exist. If the server cannot create or write to the directory, it writes one line to stderr and starts. It then logs only to the console.

### Console output

Standard output now contains only the output of CLI commands. The server log is on stderr. Update any supervisor unit, container configuration, or shell pipeline that read the server log from stdout. Docker, systemd and the Java mod capture both streams and need no change.

When stderr is not a terminal, the server does not write colour codes. `NO_COLOR=1` turns colour off. `CLICOLOR_FORCE=1` turns colour on.

### Log file format and rotation

Each line is one JSON object. `instance_id` and `name` come from `server.meridian`. They are `null` when that block is absent. `boot_id` changes each time the server process starts.

```json
{"ts":"2026-08-23T18:43:32.989Z","level":"info","target":"bvc_server_lib::runtime","msg":"Protocol Version: 3.0.0","instance_id":null,"name":null,"boot_id":"01a02fef-63fc-7ab3-80f0-76853ffb4bf2","file":"server/src/runtime/mod.rs","line":164}
```

Fields that belong to one log message are top-level keys. Read them with `jq`, for example `jq '.bind'`.

The log file rotates by size:

| Before | Now |
|---|---|
| `bvc-server.log.2026-08-23` | `bvc-server.log` plus `bvc-server_<timestamp>.log` archives |
| One file per day, kept forever | 50 MB per file, ten archives kept |

Update any logrotate rule or log collector pattern that matches the old names.

### `RUST_LOG`

`RUST_LOG` overrides `log.level` and the built-in log level of each library. It applies to log output from the server and from its libraries.

```
RUST_LOG=info,bvc_server_lib::stream=debug,hyper=off
```

A directive with no `=` sets the global level. A directive with `=` sets the level for each log target whose name starts with the text before `=`. The target is the `target` field in the JSON log. If more than one directive matches, the longest one applies. `off` turns a target off. If the server cannot read a directive, it names the directive on stderr and ignores it.

Only the `target=level` form is supported. The span and field syntax of some Rust tools (`target[span{field=value}]=level`) is not supported.

## Scripts that call the API

Three API changes affect your own scripts. Update any script that uses these routes. The 1.0.0-beta.21 Addon, Java mods and client already use the new form.

| Route | Change |
|---|---|
| `/api/position`, `/api/control`, `/api/state`, `/api/preferences`, `/api/audio/event`, the chat WebSocket | Send `Authorization: Bearer <token>` instead of `X-MC-Access-Token: <token>`. The token value does not change. |
| `POST /api/bedrock/transfer` | Removed with the transfer relay. Delete the call. The client no longer needs it. |
| `GET /api/config` | `bedrock.transfer_port` is removed. `bedrock.enabled` stays and is always `true`. |

## QUIC ports

`server.advertised_quic_ports` sets the UDP ports that a client tries, in order of preference. If the list is not empty, clients try only the ports in the list. They do not try `server.quic_port` unless it is in the list. An empty list publishes only `quic_port`.

File: `config.hcl`

```hcl
server {
    quic_port = 443
    advertised_quic_ports = [443, 8443]
}
```

Use this list when a proxy receives traffic on a port that is not the bind port. The server binds one socket, however many ports it publishes.

The environment variable is `BVC_ADVERTISED_QUIC_PORTS`.

## WebSocket voice transport

Voice can now also use WebSocket on the HTTPS port. WebSocket uses `server.port` (443 by default) and needs no firewall change. The server always advertises it on `/api/config`. No key turns it off.

A client tries the published QUIC ports first. If it cannot connect on any QUIC port, it uses WebSocket.

### Forcing WebSocket-only voice

No setting turns off QUIC. To force WebSocket, make QUIC unreachable.

File: `config.hcl`

```hcl
server {
    quic_port = 0
    advertised_quic_ports = [1]
}
```

`quic_port = 0` binds the QUIC listener to a port that the operating system selects. `advertised_quic_ports = [1]` publishes a port where nothing listens. Every QUIC attempt fails, and voice uses WebSocket on 443.

:::note
Each client tries all published QUIC ports and waits for them to fail before it uses WebSocket. Sessions start more slowly.
:::

## Peering

:::caution
Peering is experimental and can change. Do not build anything on the peering protocol until a formal specification is announced.
:::

Peering is new in this release. A 1.0.0-beta.20 peer link does not work in 1.0.0-beta.21.

| 1.0.0-beta.20 | 1.0.0-beta.21 |
|---|---|
| `relay` config block | `server.peers` blocks and `server.peer_relay_url` |
| `POST /api/relay/offer` | Removed |
| `POST /api/relay/peer_link` | `GET /api/admin/relay/peerlink`, or `bvc relay peerlink` |
| `POST /api/relay/peer_redeem` | Removed |
| `peer_link` player permission | Removed. A peer declared in `config.hcl` is allowed to connect. |
| Certificates with the name `server::{host}:{port}` | The server refuses the connection. |

Link each pair of servers again. Run `bvc relay peerlink` on each server, and paste the printed `peers` block into the other server's `config.hcl`. See [peering](/wiki/reference/peering/).

The default capability is `carry_speakers` only. A peer gets `query_audio` and `serve_audio` only if you name them. An empty `capabilities` list gives the peer nothing.

If two peers cannot connect directly, set `server.peer_relay_url`.

:::note
`server.peer_relay_url` is removed after beta.21. After beta.21, peers must connect directly. There is no relay. See [peering](/wiki/reference/peering/).
:::

## Java mod

`embedded-config` now uses the same keys as the BVC server's own configuration. The flat keys are removed.

:::caution
The embedded server **does not start** while an old key is present. It logs each old key with its replacement and stops. The mod does not change the file.
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

The camelCase spellings (`httpPort`, `tlsCertificate`, and the others) change in the same way.

The same configuration as nested blocks:

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

A key that you do not set uses the server default. See the [Java mod](/wiki/server/java-mod/) pages.

## Bedrock Addon

Each entry in `server.bedrock.servers` must set `addon_mode`. It selects how position, state and chat events for the world reach the BVC server.

| Value | Use for |
|---|---|
| `net` | A world that runs the standard Bedrock Addon or the Java mod. The Addon or the mod sends events to the BVC server over HTTP. |
| `no_net` | A Realms or Aternos world that runs the [no-net Addon](/wiki/server/nonet-addon/). The BVC client reads the events from the game traffic and sends them. |

The `BVC_BEDROCK_SERVERS` environment variable accepts an optional last token, `@net` or `@no_net`:

```
Name@host[:port][@protocol][@net|@no_net]
```

The server identifies each of the last two tokens from its value: a number is the protocol, and `net` or `no_net` is the mode. You can omit either token and write them in either order. See [environment variables](/wiki/reference/environment-variables/#bedrock).

## Hytale

The Hytale Java mod is removed, with its build and release pipeline.