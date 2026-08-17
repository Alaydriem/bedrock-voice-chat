---
title: Upgrading to 1.0.0-beta.21
description: Voice protocol 3.0.0, the peering rewrite, and every configuration key that changed since 1.0.0-beta.20.
sidebar:
  label: 1.0.0-beta.21
  order: 2
---

Bedrock Voice Chat 1.0.0-beta.21 changes the voice protocol, replaces the peering system, and moves several configuration keys.

## Before you start

:::danger
Back up the database **first**. The `drop_peer_link_permission` migration deletes every `peer_link` grant, and its rollback restores nothing. Which players held the grant is recorded nowhere else.
:::

## Voice protocol 3.0.0

The voice protocol moves from `2.1.0` to `3.0.0`. Clients on 1.0.0-beta.20 and earlier cannot open a session against a 1.0.0-beta.21 server, and a 1.0.0-beta.21 client cannot open one against an older server. Only the major and minor are compared.

Publish the client update alongside the server upgrade.

## Configuration changes

File: `config.hcl`

| Key | Change | Action |
|---|---|---|
| `server.bedrock.servers[].addon_mode` | New. **Required** on every entry. | Add `addon_mode = "net"` or `addon_mode = "no_net"` to each entry. |
| `server.bedrock.dns` | Removed. | Delete the block. |
| `relay` | Removed. Replaced by `server.peers` and `server.peer_relay_url`. | Delete the block. Redeclare peers. |
| `voice.recording` | New. `enabled` defaults to `true`. | Set `enabled = false` to forbid client-side recording. |

`addon_mode` has no default. A `servers` list carrying an entry without it does not parse, and the server does not start. Every key is listed in the [configuration reference](/wiki/reference/configuration/).

:::caution
Every other removed key is **ignored, not rejected**. A leftover `relay` or `server.bedrock.dns` block starts the server with those settings absent and writes nothing to the log.
:::

`voice.recording.enabled` takes the `BVC_RECORDING` environment override.

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

Relink each pair of servers. Run `bvc relay peerlink` on each side and paste the printed `peer` block into the other's `config.hcl`. See [peering](/wiki/reference/peering/).

Capabilities default to `carry_speakers` alone. `query_audio` and `serve_audio` are granted only where named. An explicitly empty `capabilities` list stays empty.

Where two peers have no direct path, set `server.peer_relay_url`. See [peer relay](/wiki/server/peer-relay/).

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
