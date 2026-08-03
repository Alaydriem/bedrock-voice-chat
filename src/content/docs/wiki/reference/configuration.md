---
title: Configuration reference
description: Every block and key in config.hcl, with defaults.
sidebar:
  label: config.hcl
  order: 2
---

Complete reference for `config.hcl`. Almost nothing here is required; for a working starting point see [Installing the BVC server](/wiki/server/installation/).

Every key also has an environment override; see [environment variables](/wiki/reference/environment-variables/). Precedence is **environment > config file > default**.

## Environment interpolation

`${env.VAR}` is evaluated anywhere in the file:

```hcl
minecraft {
    access_token = "${env.BVC_ACCESS_TOKEN}"
}
```

A referenced variable that is unset is a hard startup error, never an empty string. A missing secret stops the server rather than booting it unsecured.

## `server`

| Key | Default | Description |
|---|---|---|
| `listen` | `::` | Bind address. The IPv6 wildcard also serves IPv4 peers. |
| `port` | `443` | HTTP/REST and login. |
| `quic_port` | `443` | UDP/QUIC voice transport. |
| `advertised_quic_ports` | `[]` | Additional public UDP ports clients should try, in preference order. Empty means only `quic_port`. Use when a proxy fronts the server on a different port. |
| `assets_path` | `./assets` | Static asset directory served by the API. |

The default `::` gives QUIC a dual-stack socket on every platform. Rocket's HTTP listener cannot be dual-stack on Windows, so there it falls back to `0.0.0.0` automatically and logs that it did. IPv6 HTTP is unavailable on Windows; IPv4 clients are unaffected. On Linux this follows `net.ipv6.bindv6only`, which defaults to `0`.

## `server.tls`

| Key | Default | Description |
|---|---|---|
| `certificate` | — | Path to a fullchain PEM. Required unless `acme` is set. |
| `key` | — | Path to the private key PEM. Required unless `acme` is set. |
| `names` | `["localhost"]` | SAN hostnames clients use to reach the server. |
| `ips` | `["127.0.0.1"]` | SAN IPs clients use to reach the server. |
| `certs_path` | `./certificates` | Directory holding the mTLS CA (`ca.crt`) used to authenticate clients. Managed for you. |
| `so_reuse_port` | `false` | Enable `SO_REUSEPORT` on the QUIC socket. Linux only. |
| `acme` | — | Automatic certificates. Mutually exclusive with `certificate`/`key`. |

## `server.tls.acme`

Obtains and renews a certificate over ACME DNS-01. The challenge is DNS-based, so port 80 never needs to be open. The server refuses to start if this is set alongside `certificate`/`key`.

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

Provider-specific fields are validated at startup and name exactly what is missing, rather than failing at first renewal.

## `server.minecraft`

| Key | Default | Description |
|---|---|---|
| `access_token` | — | Shared secret the Addon sends as `X-MC-Access-Token`. **Required.** |
| `client_id` | `a17f9693-f01f-4d1d-ad12-1f179478375d` | Microsoft OAuth2 client ID. **Do not change** — this is the public BVC client every released app authenticates against. |

## `server.features`

| Key | Default | Description |
|---|---|---|
| `openapi_docs` | `false` | Serve `/openapi.json` and the API browser at `/docs`. |
| `telemetry` | `true` | Anonymous usage metrics. See [privacy and telemetry](/wiki/server/privacy-and-telemetry/). |
| `relay` | — | Cross-server peering tuning. See below. |

One-time code login (`POST /api/auth/code`, used by `bvc login`) is always enabled. Earlier releases gated it behind a `code_login` flag. That key no longer exists and is ignored if present.

## `server.features.relay`

Cross-server peering. Early access: functional, still being hardened, and the shape may change between releases. Gated by the `peer_link` permission. See [peering](/wiki/reference/peering/).

| Key | Default | Description |
|---|---|---|
| `announce_interval_secs` | `60` | Seconds between self-announce cycles. |
| `orchestration_interval_secs` | `5` | Seconds between offer, idle-sweep, and reconnect-grace cycles. |
| `idle_timeout_secs` | `300` | Seconds a peer link may idle before it is closed. |

The defaults suit production; these keys exist mainly so integration tests can shorten them.

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

The relay behind Proxy Connect and Realms Connect. This is how BVC supports Aternos, Realms, and consoles.

| Key | Default | Description |
|---|---|---|
| `enabled` | `true` | Master switch. When off, `/api/config` advertises nothing for it. |
| `transfer_port` | `19132` | Port players point Minecraft at. |
| `transfer_target_port` | `19137` | Internal port sessions are handed off to. |
| `transfer_cache_ttl_secs` | `900` | How long a resolved transfer target is cached. |
| `proxy_event_freshness_threshold_secs` | `30` | Age beyond which a proxied position event is discarded. |
| `servers` | `[]` | Curated server list advertised to clients. |

### `server.bedrock.dns`

| Key | Default | Description |
|---|---|---|
| `enabled` | **`false`** | DNS responder. Consoles cannot connect without it. |
| `port` | `53` | Bind port. Needs elevated privileges, or `CAP_NET_BIND_SERVICE` on Linux. |
| `upstream` | `["1.1.1.1", "1.0.0.1"]` | Resolvers for everything not overridden. |
| `override_host` | `geo.hivebedrock.network` | Hostname redirected to this server. |
| `rate_limit_per_sec` | `100` | Per-client query rate limit. |

A console cannot type a custom server address, so it reaches BVC by having its DNS point here. With `dns.enabled = false` that path does not work at all. See [console and mobile](/wiki/platforms/console-and-mobile/).

### `server.bedrock.servers`

```hcl
bedrock {
    servers = [
        {
            name             = "My SMP"
            host             = "play.example.com"
            port             = 19132
            protocol_version = 1001
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

Database TLS is validated at startup rather than degrading silently:

- Any `ssl_*` path without `ssl_mode` is an error — sqlx would otherwise default to opportunistic TLS and fall back to plaintext.
- `ssl_root_cert` with a mode weaker than `verify-ca` is an error, because the pinned CA would never be checked.
- `ssl_cert` and `ssl_key` must be set together, and every referenced file must exist.
- Any `ssl_*` option on a SQLite scheme is an error.

## `log`

| Key | Default | Description |
|---|---|---|
| `level` | `info` | `trace`, `debug`, `info`, `warn`, `error`. |
| `out` | `stdout` | Log destination. |

## `voice`

| Key | Default | Description |
|---|---|---|
| `datagram_send_capacity` | `1024` | Outbound QUIC datagram queue per connection. |
| `datagram_recv_capacity` | `1024` | Inbound QUIC datagram queue per connection. |

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
        peer_link    = false
    }
}
```

Server-wide defaults for the four permissions. Per-player overrides are managed with the `permission` CLI subcommands — see the [CLI reference](/wiki/reference/cli/).

| Permission | Grants |
|---|---|
| `audio_upload` | Upload clips to the audio library. |
| `audio_delete` | Delete clips from the library. |
| `admin` | User and permission management. |
| `peer_link` | Establish cross-server peer links. Early access. |
