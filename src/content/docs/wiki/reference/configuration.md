---
title: Configuration reference
description: Every block and key in config.hcl, with defaults.
sidebar:
  label: config.hcl
  order: 2
---

The Bedrock Voice Chat server reads these blocks and keys from `config.hcl`. Almost no key is required. For a working example, see [Installing the BVC server](/wiki/server/installation/).

Most keys also have an environment variable. See [environment variables](/wiki/reference/environment-variables/). Precedence: **environment, then config file, then default**.

The server reads `config.hcl` one time, at startup. Restart the server after you edit it.

## Environment interpolation

You can use `${env.VAR}` anywhere in the file:

```hcl
minecraft {
    access_token = "${env.BVC_ACCESS_TOKEN}"
}
```

If a referenced variable is not set, the server does not start.

## `server`

| Key | Default | Description |
|---|---|---|
| `listen` | `::` | Bind address. The IPv6 wildcard also accepts IPv4 connections. |
| `port` | `443` | Public TCP port for the HTTP API and login. The server accepts TLS on this port and forwards API requests to an internal listener on a loopback port. |
| `quic_port` | `28280` | UDP port for QUIC voice. |
| `advertised_quic_ports` | `[28280]` | Public UDP ports that clients try, in order of preference. Clients get only the ports in this list. `quic_port` is not added. The default stays `[28280]` when you change `quic_port`. Set this list when you change `quic_port`, or when a proxy receives traffic on a different port. An empty list publishes only `quic_port`. |
| `assets_path` | `./assets` | Directory of static files that the API serves. |

With the default `::`, QUIC uses one socket for IPv4 and IPv6. On Linux, this depends on `net.ipv6.bindv6only`, which is `0` by default.

## `server.tls`

| Key | Default | Description |
|---|---|---|
| `certificate` | None | Path to a full-chain PEM file. Required unless `acme` is set. |
| `key` | None | Path to the private key PEM file. Required unless `acme` is set. |
| `names` | `["localhost"]` | Hostnames that clients use to reach the server. The server puts them in the certificate. |
| `ips` | `["127.0.0.1"]` | IP addresses that clients use to reach the server. The server puts them in the certificate. |
| `certs_path` | `./certificates` | Directory for the mTLS CA (`ca.crt`) that authenticates clients. The server manages this directory. |
| `acme` | None | Automatic certificates. Do not use with `certificate` and `key`. |

## `server.tls.acme`

Gets and renews a certificate with ACME DNS-01. You do not need to open port 80. If you set `acme` and also `certificate` or `key`, the server does not start.

| Key | Default | Description |
|---|---|---|
| `email` | None | ACME account contact email. **Required.** |
| `provider` | None | `cloudflare` or `acme-dns`. **Required.** |
| `api_token` | None | Cloudflare only. An API token for one zone, with DNS edit permission. |
| `server_url` | None | acme-dns only. |
| `username` | None | acme-dns only. |
| `password` | None | acme-dns only. |
| `subdomain` | None | acme-dns only. |
| `directory` | `https://acme-v02.api.letsencrypt.org/directory` | ACME directory URL. Use the staging directory during tests to avoid rate limits. |
| `domains` | DNS entries of `tls.names` | Certificate domains. The server skips IP entries in `tls.names`. An ACME certificate cannot contain an IP address. |

The server checks the provider fields at startup. If a field is missing, the error names it.

## `server.minecraft`

| Key | Default | Description |
|---|---|---|
| `access_token` | None | Shared secret that the Addon sends as `Authorization: Bearer`. **Required.** |
| `client_id` | `a17f9693-f01f-4d1d-ad12-1f179478375d` | Microsoft OAuth2 client ID. **Do not change.** All released apps authenticate with this public BVC client ID. |

## `server.features`

| Key | Default | Description |
|---|---|---|
| `chat` | `true` | Sends in-game chat between the game and the app. See [Chat](/wiki/player/chat/). |
| `openapi_docs` | `false` | Serves `/openapi.json` and the API browser at `/docs`. |
| `radar` | `true` | Shows players beyond voice range on the dashboard ring. See [radar](#radar). |
| `telemetry` | `true` | Anonymous usage metrics. See [privacy and telemetry](/wiki/server/privacy-and-telemetry/). |

With `chat = false`, the server sends no chat in either direction. The server drops chat messages from the Addon and refuses chat messages from the app. The Addon's chat connection still opens, but it is not used. You do not change the Addon or mod configuration.

The app reads the `chat` setting when it connects. The chat panel stays on the dashboard, greyed out, and shows `Chat is disabled on this server`.

### `radar`

The dashboard ring shows players who are beyond voice range. The server still sends the positions of these players to the client. With `radar = false`, the ring is empty. The player list does not change.

`radar` changes only the display. A modified client can still show those players.

The server publishes this value on `/api/config` as `radar.enabled`. If a client cannot read the config, it shows the ring, the same as with an older server.

### Code login

One-time code login (`POST /api/auth/code`, used by `bvc login`) is always on. Earlier releases had a `code_login` flag for it. That key no longer exists. The server ignores it.

## `server.peers`

Declared peers, one labelled block for each peer. The server does not find peers automatically. It refuses a connection from any server that is not in this list.

:::caution[Work in progress]
The format of this block changes between releases. See [peering](/wiki/reference/peering/).
:::

```hcl
server {
    peers "partner-smp" {
        peerlink     = "<base32 peer link>"
        worlds       = ["overworld"]
        capabilities = ["carry_speakers"]
    }
}
```

| Key | Default | Description |
|---|---|---|
| `peerlink` | None | The peer link that the other server printed. It contains that server's public key and its network addresses. **Required.** |
| `worlds` | `[]` | Limits the link to some of the peer's worlds. An empty list accepts all worlds that the peer declares. |
| `capabilities` | `["carry_speakers"]` | The actions that the peer can do on the link. |

Get `peerlink` from the other server with `bvc relay peerlink`. The command prints a block that you can paste. See the [CLI reference](/wiki/reference/cli/).

Log lines use the block label as the name of the peer.

### `worlds`

During the handshake, a peer declares the worlds that it hosts. `worlds` limits which of those worlds this server accepts.

| Value | Effect |
|---|---|
| `[]` | Accept every world that the peer declares. |
| `["a", "b"]` | Accept only the worlds that are in this list and in the peer's declaration. |

`worlds` cannot add a world that the peer did not declare.

### `capabilities`

| Value | Effect |
|---|---|
| `carry_speakers` | Sends voice across the link. |
| `query_audio` | Reserved. Has no effect. |
| `serve_audio` | Reserved. Has no effect. |

If you omit the key, the peer gets `carry_speakers`. An empty list gives the peer nothing.

If a value is not recognised, the server does not start, and the error names the value.

Peers do not fetch audio files. The server that plays Jukebox audio decodes it and sends it to peers as normal voice audio.

## `server.enrollment`

`server.enrollment` gets a hostname and a certificate for a server that has no domain. See [assigned hostnames](/wiki/server/enrollment/).

| Key | Default | Description |
|---|---|---|
| `token` | None | The single-use enrollment token. The server uses it at first start. |
| `address` | None | The address to publish as the A record of the assigned name. |

Use only one certificate source: `enrollment.token`, `tls.acme`, or `tls.certificate` with `tls.key`. If you set more than one, the server does not start.

`address` is optional. If you do not set it, BVC publishes no DNS record. BVC clients can reach the server without the record. Only a Bedrock Addon with `addon_mode = "net"` on a different host needs the name to resolve. BVC publishes a private address but does not check it. BVC checks a public address every day.

### Registry address

The BVC registry address is compiled into the binary. No config key or environment variable changes it. To use a different registry, rebuild the server.

## `server.peering` and `server.peer_port`

| Key | Default | Description |
|---|---|---|
| `peering` | `true` | Binds the peer endpoint when no peer is declared. Another server can then pair with this server. |
| `peer_port` | `28284` | UDP port of the peer endpoint. `0` asks the system for a random port. |

With `peering = false`, the peer endpoint binds only when `server.peers` declares at least one peer.

The peer link contains this port. A random port changes at each restart, and the peer link that the other server has becomes invalid. For this reason, the default is a fixed port. Open or forward this port when peers connect to this server directly.

## `server.cors`

| Key | Default | Description |
|---|---|---|
| `allowed_origins` | `[]` | Allowed origins. An empty list allows all origins. The BVC client uses mTLS and is not affected. |
| `allow_credentials` | `true` | Sends `Access-Control-Allow-Credentials`. |

## `server.age`

The minimum age for your community. Clients use it to decide when to show Age Signals (Android) or Declared Age Ranges (iOS) prompts.

| Key | Default | Description |
|---|---|---|
| `minimum` | `13` | Minimum age. `0` turns off the age check. The server changes any other value to the nearest value from 13 to 18. |

## `server.bedrock`

The server list that clients see in Bedrock Voice Chat Connect, and the maximum age of position events from Connect.

Bedrock support is always on. No key turns it off.

| Key | Default | Description |
|---|---|---|
| `proxy_event_freshness_threshold_secs` | `30` | The server discards a position event from Connect that is older than this number of seconds. |
| `servers` | `[]` | The server list that the server sends to clients. |

### `server.bedrock.servers`

```hcl
bedrock {
    servers = [
        {
            name             = "My SMP"
            host             = "play.example.com"
            port             = 19132
            protocol_version = 1001
            addon_mode       = "net"
        }
    ]
}
```

| Key | Default | Description |
|---|---|---|
| `name` | None | Name shown in the client. |
| `host` | None | Hostname of the Bedrock server. |
| `port` | `19132` | Port of the Bedrock server. |
| `protocol_version` | None | The protocol version that the BVC client shows to Minecraft. If you omit it, the client uses the version of the Bedrock server (Auto). See [version support](/wiki/platforms/version-support/). |
| `addon_mode` | None | `net` or `no_net`. **Required.** Selects how events for this world reach the BVC server. |

`addon_mode` selects how position, state and chat events for the world reach the BVC server.

| Value | Delivery |
|---|---|
| `net` | The world's BVC Addon sends events to the BVC server over HTTP. Connect only forwards game traffic. |
| `no_net` | The world has no HTTP connection. Connect reads the events from the game traffic and sends them. |

You must set `addon_mode`. The server cannot detect it. The server identifies a running Addon by the UUID of its world, and it cannot match that UUID to an entry in `servers`.

`addon_mode` does not change how a client connects to the BVC server.

`addon_mode` has no default. Each entry must set it. If one entry does not set it, the server cannot read the `servers` list.

Use `no_net` for Realms and Aternos worlds that run the no-net Addon. Use `net` for a world that runs the standard Addon or the Java mod.

## `database`

| Key | Default | Description |
|---|---|---|
| `scheme` | `sqlite3` | `sqlite`, `sqlite3`, `mysql`, `postgres`, or `postgresql`. |
| `database` | `./bvc.sqlite3` | SQLite path, or database name. |
| `host` | `127.0.0.1` | MySQL and Postgres only. |
| `port` | `3306` / `5432` | Default for each scheme. |
| `username` | None | MySQL and Postgres only. |
| `password` | None | MySQL and Postgres only. |
| `ssl_mode` | None | `disable`, `prefer`, `require`, `verify-ca`, `verify-full`. |
| `ssl_root_cert` | None | CA certificate that verifies the database server. |
| `ssl_cert` | None | Client certificate for database mTLS. |
| `ssl_key` | None | Key for `ssl_cert`. |

The server checks the database TLS settings at startup. It rejects these combinations:

- Any `ssl_*` path without `ssl_mode`. Without `ssl_mode`, the connection can fall back to plaintext.
- `ssl_root_cert` with a mode weaker than `verify-ca`. The server would never check the CA.
- `ssl_cert` without `ssl_key`, or a path to a file that does not exist.
- Any `ssl_*` option with a SQLite scheme.

## `log`

| Key | Default | Description |
|---|---|---|
| `level` | `info` | `trace`, `debug`, `info`, `warn`, `error`. `RUST_LOG` overrides it. |
| `path` | `./logs` | Directory for the rotating JSON log file. The server creates it if it does not exist. |

You cannot configure the console output. The server always writes human-readable output to stderr, and a JSON file to `log.path`. If the server cannot write to `log.path`, it writes logs only to the console and continues to run.

## `voice`

| Key | Default | Description |
|---|---|---|
| `datagram_send_capacity` | `1024` | Size of the outgoing QUIC datagram queue for each connection. |
| `datagram_recv_capacity` | `1024` | Size of the incoming QUIC datagram queue for each connection. |
| `send_batch_wait_micros` | `7500` | Microseconds that the sender waits after the first queued datagram before it sends the batch. `0` sends each datagram immediately. |

A non-zero `send_batch_wait_micros` adds up to that many microseconds of latency. The server then sends fewer, larger UDP packets and uses less CPU. The client's playback buffer hides the default delay.

### `voice.recording`

| Key | Default | Description |
|---|---|---|
| `enabled` | `true` | Whether clients connected to this server can record. |

The client applies this setting. The server does not. Recordings are saved on the player's own computer and never go to the server. When `false`, the standard client does not start a recording from the REC button, `/bvc:record`, or the WebSocket API. A modified client can still record.

The server publishes this value on `/api/config`. If a client cannot read the config, it allows recording, the same as with an older server.

See [recording](/wiki/creator/recording/).

### `voice.limits`

| Key | Default | Description |
|---|---|---|
| `connections` | `0` | Maximum number of voice sessions at the same time. `0` means no limit. |
| `reconnect_grace` | `60` | Number of seconds that a disconnected player keeps their slot. |

The limit counts players. One player on two devices uses one slot. The limit never refuses a player who reconnects, even while the old connection closes.

When the limit refuses a player, the client shows a notice about the limit and tries again. The player connects when a slot is free. The limit does not affect registration. New players are still added to the player list, and can sign in when there is a free slot.

`reconnect_grace` keeps the slot of a player who disconnected cleanly. Examples: the player closed the app, or Android stopped it in the background. A new player who arrives during that time does not take the slot. When no other slot is free, a new player gets a held slot. The oldest held slot is released first.

The server publishes these values on `/api/config` as `capacity.limit` and `capacity.in_use`. If a client cannot read the config, it treats the server as unlimited, the same as with an older server.

Players from a peer server do not use slots on this server. Their own server counts them. The limit does not apply to the HTTP API. You can administer a full server.

### `voice.spatial_audio`

All distances are in blocks.

| Key | Default | Description |
|---|---|---|
| `broadcast_range` | `48.0` | Maximum distance at which the server sends audio. |
| `close_threshold` | `24.0` | Full volume at this distance or closer. |
| `falloff_distance` | `48.0` | Distance at which the volume reaches its minimum. |
| `steepen_start` | `38.0` | Distance where the volume drops faster towards silence. |
| `deafen_distance` | `3.0` | At distances below this, there is no stereo panning. |
| `panning_start` | `8.0` | Distance at which stereo panning starts to increase. |
| `max_attenuation_db` | `40.0` | Volume reduction at `falloff_distance`, in dB. |

Panning increases from `panning_start` to `close_threshold`. A higher `close_threshold` spreads voices across the stereo field over a wider range of distances.

## `audio`

| Key | Default | Description |
|---|---|---|
| `file_path` | `./assets/audio` | Directory for uploaded audio clips. |
| `max_concurrent_per_uuid` | `5` | Maximum number of clips that one player can play at the same time. |

## `permissions`

```hcl
permissions {
    defaults = {
        audio_upload = true
        audio_delete = false
        admin        = false
    }
}
```

The server-wide defaults for the three permissions. Set overrides for one player with the `permission` CLI commands. See the [CLI reference](/wiki/reference/cli/).

| Permission | Grants |
|---|---|
| `audio_upload` | Upload clips to the audio library. |
| `audio_delete` | Delete clips from the library. |
| `admin` | Manage players and permissions. |

Peering is not a player permission. You declare a peer in [`server.peers`](/wiki/reference/configuration/#serverpeers).
