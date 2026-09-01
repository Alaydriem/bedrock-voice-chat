---
title: Configuration reference
description: Every block and key in config.hcl, with defaults.
sidebar:
  label: config.hcl
  order: 2
---

Every block and key Bedrock Voice Chat server accepts in `config.hcl`, with defaults. Almost nothing here is required. For a working starting point see [Installing the BVC server](/wiki/server/installation/).

Every key also has an environment override; see [environment variables](/wiki/reference/environment-variables/). Precedence is **environment > config file > default**.

`config.hcl` is read once at startup. Restart the server after editing it.

## Environment interpolation

`${env.VAR}` is evaluated anywhere in the file:

```hcl
minecraft {
    access_token = "${env.BVC_ACCESS_TOKEN}"
}
```

An unset referenced variable is a hard startup error. The server stops instead of booting unsecured.

## `server`

| Key | Default | Description |
|---|---|---|
| `listen` | `::` | Bind address. The IPv6 wildcard also serves IPv4 peers. |
| `port` | `443` | HTTP/REST and login. The public port. The API listener binds a loopback port behind the TLS demultiplexer. |
| `quic_port` | `443` | UDP/QUIC voice transport. |
| `advertised_quic_ports` | `[443, 28280]` | Public UDP ports clients should try, in preference order. The list replaces `quic_port` as the published set; it does not extend it. Set it explicitly when a proxy fronts the server on a different port, or when `quic_port` is moved — the default does not follow the bind port. An explicitly empty list publishes `quic_port` alone. |
| `assets_path` | `./assets` | Static asset directory served by the API. |

The default `::` gives QUIC a dual-stack socket on every platform. On Linux this follows `net.ipv6.bindv6only`, which defaults to `0`.

## `server.tls`

| Key | Default | Description |
|---|---|---|
| `certificate` | — | Path to a fullchain PEM. Required unless `acme` is set. |
| `key` | — | Path to the private key PEM. Required unless `acme` is set. |
| `names` | `["localhost"]` | SAN hostnames clients use to reach the server. |
| `ips` | `["127.0.0.1"]` | SAN IPs clients use to reach the server. |
| `certs_path` | `./certificates` | Directory holding the mTLS CA (`ca.crt`) used to authenticate clients. Managed for you. |
| `acme` | — | Automatic certificates. Mutually exclusive with `certificate`/`key`. |

## `server.tls.acme`

Obtains and renews a certificate over ACME DNS-01. Port 80 never needs to be open. The server refuses to start with this set alongside `certificate`/`key`.

| Key | Default | Description |
|---|---|---|
| `email` | — | ACME account contact. **Required.** |
| `provider` | — | `cloudflare` or `acme-dns`. **Required.** |
| `api_token` | — | Cloudflare only. Zone-scoped token with DNS edit permission. |
| `server_url` | — | acme-dns only. |
| `username` | — | acme-dns only. |
| `password` | — | acme-dns only. |
| `subdomain` | — | acme-dns only. |
| `directory` | `https://acme-v02.api.letsencrypt.org/directory` | ACME directory URL. Point at the staging directory while testing to avoid rate limits. |
| `domains` | DNS entries of `tls.names` | Explicit certificate domains. IP entries in `tls.names` are skipped — they cannot appear on an ACME certificate. |

Provider-specific fields are validated at startup. A missing one is named in the error.

## `server.minecraft`

| Key | Default | Description |
|---|---|---|
| `access_token` | — | Shared secret the Addon sends as `Authorization: Bearer`. **Required.** |
| `client_id` | `a17f9693-f01f-4d1d-ad12-1f179478375d` | Microsoft OAuth2 client ID. **Do not change** — this is the public BVC client every released app authenticates against. |

## `server.features`

| Key | Default | Description |
|---|---|---|
| `chat` | `true` | Relay in-game chat between the game and the app. See [Chat](/wiki/player/chat/). |
| `openapi_docs` | `false` | Serve `/openapi.json` and the API browser at `/docs`. |
| `telemetry` | `true` | Anonymous usage metrics. See [privacy and telemetry](/wiki/server/privacy-and-telemetry/). |

With `chat = false` the server relays nothing in either direction. Lines reported by the Addon are dropped, and messages sent from the app are refused. The Addon's chat socket still connects and then stays idle. No Addon or mod configuration changes.

The app reads this at connect. The chat dock stays on the dashboard, greyed out, and reports `Chat is disabled on this server`.

One-time code login (`POST /api/auth/code`, used by `bvc login`) is always enabled. Earlier releases gated it behind a `code_login` flag. That key no longer exists and is ignored if present.

## `server.peers`

Declared peers, one labeled block each. Peering is declared, never discovered. A node absent from this list is refused at connect.

Work in progress. The shape changes between releases. See [peering](/wiki/reference/peering/).

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
| `peerlink` | — | The link the far side printed. Carries the peer's public key and the paths to reach it. **Required.** |
| `worlds` | `[]` | Narrowing filter. Empty accepts what the peer declares. |
| `capabilities` | `["carry_speakers"]` | What the peer may do on the link. |

Get `peerlink` from the other server with `bvc relay peerlink`. It prints a block ready to paste. See the [CLI reference](/wiki/reference/cli/).

The block label names the peer in log lines.

### `worlds`

A peer declares which worlds it hosts during the handshake. `worlds` narrows what this server accepts from that declaration.

| Value | Effect |
|---|---|
| `[]` | Accept every world the peer declares. |
| `["a", "b"]` | Accept only the intersection of that list and the peer's declaration. |

It never grants a world the peer did not declare.

### `capabilities`

| Value | Effect |
|---|---|
| `carry_speakers` | Carry voice across the link. |
| `query_audio` | Reserved. Granting it changes no behaviour. |
| `serve_audio` | Reserved. Granting it changes no behaviour. |

Omitting the key grants `carry_speakers`. An explicitly empty list stays empty and grants nothing.

An unrecognised value stops startup and names the value.

`query_audio` and `serve_audio` described a peer fetching an audio file it did not hold. Nothing does that now. Jukebox audio is decoded where it is played and reaches a peer as ordinary frames.

## `server.enrollment`

Getting a hostname and a certificate without owning a domain. See [assigned hostnames](/wiki/server/enrollment/).

| Key | Default | Description |
|---|---|---|
| `token` | — | The single-use enrollment token. Spent on first boot. |
| `address` | — | Address to publish as the assigned name's A record. |

Setting `token` alongside `tls.acme`, `tls.certificate` or `tls.key` stops startup. The three are mutually exclusive.

`address` is optional and unset publishes no record. BVC clients reach the server without one; only a `net`-mode Bedrock addon on another host needs the name to resolve. A private address is published but never verified. A public one is re-checked daily.

### There is no `server.registry`

Where a server reaches the BVC registry is compiled into the binary. It is not a config key and not a runtime environment variable, so a released build cannot be pointed at a different registry — that takes a rebuild. Nothing an operator writes changes it.

## `server.peer_port`

| Key | Default | Description |
|---|---|---|
| `peer_port` | `28284` | UDP port the peer endpoint binds. `0` requests an ephemeral port. |

The peer link carries this port. An ephemeral port changes on every restart, which invalidates a link the far side has already been given, so the port is fixed by default. Open or forward it wherever peers reach this server directly.

## `server.cors`

| Key | Default | Description |
|---|---|---|
| `allowed_origins` | `[]` | Permitted origins. Empty allows all, which is harmless for the native mTLS client. |
| `allow_credentials` | `true` | Send `Access-Control-Allow-Credentials`. |

## `server.age`

Minimum age for your community. Clients use this to decide whether to show Age Signals (Android) or Declared Age Ranges (iOS) prompts.

| Key | Default | Description |
|---|---|---|
| `minimum` | `13` | Minimum age. `0` disables enforcement. Any other value is clamped to 13–18. |

## `server.bedrock`

The relay behind Bedrock Voice Chat Connect. This is how BVC supports Aternos, Realms, and consoles.

| Key | Default | Description |
|---|---|---|
| `enabled` | `true` | Master switch. When off, `/api/config` advertises nothing for it. |
| `transfer_port` | `28283` | Relay listen port. |
| `transfer_target_port` | `28282` | Client proxy port sessions are handed off to. Defaults to the proxy's own listen port. Change it only alongside the proxy. |
| `transfer_cache_ttl_secs` | `900` | How long a resolved transfer target is cached. |
| `proxy_event_freshness_threshold_secs` | `30` | Age beyond which a proxied position event is discarded. |
| `servers` | `[]` | Curated server list advertised to clients. |

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
| `name` | — | Display name in the client. |
| `host` | — | Hostname of the real backend. |
| `port` | `19132` | Backend port. |
| `protocol_version` | — | Protocol the client proxy advertises. Omit for Auto, which mirrors the real backend. See [version support](/wiki/platforms/version-support/). |
| `addon_mode` | — | `net` or `no_net`. **Required.** Who delivers events for this world. |

`addon_mode` states who delivers position, state and chat events for that world.

| Value | Delivery |
|---|---|
| `net` | The world's own BVC Addon posts to the BVC server over HTTP. The client proxy relays only. |
| `no_net` | There is no HTTP channel. The client proxy carries events in-band. |

The operator declares it. Addon liveness is keyed on the addon's own world uuid, and nothing maps that back to a configured entry.

`addon_mode` says nothing about how a client reaches the BVC server. That is decided separately.

`addon_mode` has no default. Every entry carries one, and a `servers` list missing it does not parse.

Use `no_net` for Realms and Aternos worlds running the no-net Addon. Use `net` for a world running the standard Addon or the Java mod.

## `database`

| Key | Default | Description |
|---|---|---|
| `scheme` | `sqlite3` | `sqlite`, `sqlite3`, `mysql`, `postgres`, or `postgresql`. |
| `database` | `./bvc.sqlite3` | SQLite path, or database name. |
| `host` | `127.0.0.1` | MySQL and Postgres only. |
| `port` | `3306` / `5432` | Defaults per scheme. |
| `username` | — | MySQL and Postgres only. |
| `password` | — | MySQL and Postgres only. |
| `ssl_mode` | — | `disable`, `prefer`, `require`, `verify-ca`, `verify-full`. |
| `ssl_root_cert` | — | CA certificate verifying the database server. |
| `ssl_cert` | — | Client certificate for database mTLS. |
| `ssl_key` | — | Key for `ssl_cert`. |

Database TLS is validated at startup. These combinations are rejected:

- Any `ssl_*` path without `ssl_mode`. sqlx would default to opportunistic TLS and fall back to plaintext.
- `ssl_root_cert` with a mode weaker than `verify-ca`. The pinned CA would never be checked.
- `ssl_cert` without `ssl_key`, or either naming a file that does not exist.
- Any `ssl_*` option on a SQLite scheme.

## `log`

| Key | Default | Description |
|---|---|---|
| `level` | `info` | `trace`, `debug`, `info`, `warn`, `error`. Overridden by `RUST_LOG`. |
| `path` | `./logs` | Directory for the rotating JSON log file. Created if missing. |

The console is not configurable. The server always writes human-readable output to stderr and a JSON file to `log.path`. An unwritable `log.path` degrades to console-only rather than stopping the server.

## `voice`

| Key | Default | Description |
|---|---|---|
| `datagram_send_capacity` | `1024` | Outbound QUIC datagram queue per connection. |
| `datagram_recv_capacity` | `1024` | Inbound QUIC datagram queue per connection. |

### `voice.recording`

| Key | Default | Description |
|---|---|---|
| `enabled` | `true` | Whether clients connected to this server may record. |

This is a client policy, not a server boundary. Recordings are written on the player's own machine and never reach the server. When false, the stock client refuses to arm from the REC button, `/bvc:record`, and the WebSocket API. It does not stop a modified client.

Advertised on `/api/config`. A client that cannot reach the config treats recording as permitted, the same as an older server.

See [recording](/wiki/creator/recording/).

### `voice.limits`

| Key | Default | Description |
|---|---|---|
| `connections` | `0` | Maximum concurrent voice sessions. `0` admits everyone. |
| `reconnect_grace` | `60` | Seconds a disconnected player keeps their slot. |

Counted per player, not per connection. A player who reconnects is never refused by the limit, even while their previous connection is still closing, and one player signed in on two devices holds one slot.

A player refused by the limit sees a notice naming it, and their client keeps retrying. They connect as soon as a slot frees. Registration is unaffected: new players still join the roster and can sign in once there is room.

`reconnect_grace` holds a departed player's slot so a client that closed cleanly — an app quit, or Android stopping it in the background — is not displaced by someone who arrived meanwhile. A held slot yields to a genuinely new player, oldest first, when nothing else is free.

Advertised on `/api/config` as `capacity.limit` and `capacity.in_use`. A client that cannot reach the config treats the server as unlimited, the same as an older server.

Relayed peer players do not consume slots on this server; they are counted where their own session lives. The HTTP API is unaffected, so a full server can still be administered.

### `voice.spatial_audio`

All distances in blocks.

| Key | Default | Description |
|---|---|---|
| `broadcast_range` | `48.0` | Maximum distance at which audio is relayed at all. |
| `close_threshold` | `24.0` | Full volume at or below this distance. |
| `falloff_distance` | `48.0` | Distance at which audio reaches full attenuation. |
| `steepen_start` | `38.0` | Distance where the falloff curve steepens toward silence. |
| `deafen_distance` | `3.0` | Below this, stereo panning is suppressed entirely. |
| `panning_start` | `8.0` | Distance at which stereo panning begins to fade in. |
| `max_attenuation_db` | `40.0` | Attenuation applied at `falloff_distance`. |

Panning ramps from `panning_start` to `close_threshold`. Raising `close_threshold` widens the band over which voices spread across the stereo field.

## `audio`

| Key | Default | Description |
|---|---|---|
| `file_path` | `./assets/audio` | Where uploaded audio clips are stored. |
| `max_concurrent_per_uuid` | `5` | Maximum simultaneous clips per player. |

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

Server-wide defaults for the three permissions. Set per-player overrides with the `permission` CLI subcommands. See the [CLI reference](/wiki/reference/cli/).

| Permission | Grants |
|---|---|
| `audio_upload` | Upload clips to the audio library. |
| `audio_delete` | Delete clips from the library. |
| `admin` | User and permission management. |

Peering is not a player permission. A peer is declared in [`server.peers`](/wiki/reference/configuration/#serverpeers).
