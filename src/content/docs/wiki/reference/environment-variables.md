---
title: Environment variables
description: Every BVC_* override, and how precedence works.
sidebar:
  label: Environment variables
  order: 3
---

Every configuration value has an environment override. Precedence is **environment > config file > default**.

An unset or empty variable never overrides anything, so `FOO=` in a compose file will not blank out a configured value. A malformed value is a hard startup error rather than a silent fallback.

This makes `config.hcl` optional. With no file, the server starts from defaults plus the environment and logs that it did — which is how it is usually run under Kubernetes. See [Docker](/wiki/server/docker/).

## Server

| Variable | Sets |
|---|---|
| `BVC_SERVER` | `server.listen` and `server.port`, from a `host:port` value. URL forms (`https://…`) are ignored here — those are the CLI's `--server-url`. |
| `BVC_QUIC_PORT` | `server.quic_port` |
| `BVC_ADVERTISED_QUIC_PORTS` | `server.advertised_quic_ports`. Comma-separated. |
| `BVC_ACCESS_TOKEN` | `server.minecraft.access_token` |
| `BVC_TELEMETRY` | `server.features.telemetry`. `true` or `false`. |

## TLS

| Variable | Sets |
|---|---|
| `BVC_TLS_CERTIFICATE` | `server.tls.certificate` |
| `BVC_TLS_KEY` | `server.tls.key` |
| `BVC_TLS_CERTS_PATH` | `server.tls.certs_path` |
| `BVC_TLS_NAMES` | `server.tls.names`. Comma-separated. |
| `BVC_TLS_IPS` | `server.tls.ips`. Comma-separated. |

## ACME

Setting any of these creates the `acme` block if it is not already in the config file. When creating it from scratch, `BVC_ACME_EMAIL` and `BVC_ACME_PROVIDER` are both required — startup fails naming whichever is missing.

| Variable | Sets |
|---|---|
| `BVC_ACME_EMAIL` | Account contact |
| `BVC_ACME_PROVIDER` | `cloudflare` or `acme-dns` |
| `BVC_ACME_API_TOKEN` | Cloudflare token |
| `BVC_ACME_DIRECTORY` | ACME directory URL |
| `BVC_ACME_DOMAINS` | Certificate domains. Comma-separated. |
| `BVC_ACME_DNS_URL` | acme-dns server URL |
| `BVC_ACME_DNS_USERNAME` | acme-dns username |
| `BVC_ACME_DNS_PASSWORD` | acme-dns password |
| `BVC_ACME_DNS_SUBDOMAIN` | acme-dns delegated subdomain |

See [TLS](/wiki/server/tls/).

## Database

| Variable | Sets |
|---|---|
| `BVC_DATABASE_SCHEME` | `database.scheme` |
| `BVC_DATABASE_DATABASE` | `database.database` |
| `BVC_DATABASE_HOST` | `database.host` |
| `BVC_DATABASE_PORT` | `database.port` |
| `BVC_DATABASE_USERNAME` | `database.username` |
| `BVC_DATABASE_PASSWORD` | `database.password` |
| `BVC_DATABASE_SSL_MODE` | `database.ssl_mode` |
| `BVC_DATABASE_SSL_ROOT_CERT` | `database.ssl_root_cert` |
| `BVC_DATABASE_SSL_CERT` | `database.ssl_cert` |
| `BVC_DATABASE_SSL_KEY` | `database.ssl_key` |

TLS combinations are validated at startup — see the [database section](/wiki/reference/configuration/#database) for which combinations are rejected.

## Bedrock relay

| Variable | Sets |
|---|---|
| `BVC_BEDROCK_ENABLED` | `server.bedrock.enabled` |
| `BVC_BEDROCK_TRANSFER_PORT` | `server.bedrock.transfer_port` |
| `BVC_BEDROCK_SERVERS` | Replaces the curated list entirely. |

`BVC_BEDROCK_SERVERS` takes comma-separated `Name@host[:port][@protocol]` entries:

```
BVC_BEDROCK_SERVERS="My SMP@play.example.com:19132@1001,Other@mc.example.net"
```

Hostnames only. An IPv6 literal's colons parse as a port separator.

## Meridian

`BVC_MERIDIAN_*` configures registration with Meridian, the proxy behind our
managed hosting. Not applicable to a self-hosted server.

## CLI variables

These are read by the CLI, not the server:

| Variable | Used by |
|---|---|
| `BVC_SERVER` | `--server-url` for `login`. Doubles as the listen override above, distinguished by whether the value is a URL. |
| `BVC_IDENTITY` | `--identity`, as `<gamertag>:<game>` |
| `BVC_GAMERTAG` | `login` only |

See [CLI](/wiki/reference/cli/).

## Interpolation instead

`${env.VAR}` works anywhere inside `config.hcl`, which is often clearer than an override:

```hcl
minecraft {
    access_token = "${env.BVC_ACCESS_TOKEN}"
}
```

A referenced variable that is unset is a hard startup error, never an empty string.
