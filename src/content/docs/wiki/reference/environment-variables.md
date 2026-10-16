---
title: Environment variables
description: Every BVC_* override, and how precedence works.
sidebar:
  label: Environment variables
  order: 3
---

Most Bedrock Voice Chat configuration values have a `BVC_*` environment variable. With these variables, you can run a server without a config file. Precedence: **environment, then config file, then default**.

A variable that is not set, or that is empty, does not change a value. `FOO=` in a compose file does not clear a configured value. If a variable has an invalid value, the server does not start.

`config.hcl` is optional. With no file, the server uses the defaults and the environment variables, and writes a log line that says so. Kubernetes deployments usually work this way. See [Docker](/wiki/server/docker/).

## Server

| Variable | Sets |
|---|---|
| `BVC_SERVER` | `server.listen` and `server.port`, from a `host:port` value. The server ignores a URL value (`https://…`). The CLI reads a URL value as `--server-url`. |
| `BVC_QUIC_PORT` | `server.quic_port` |
| `BVC_ADVERTISED_QUIC_PORTS` | `server.advertised_quic_ports`. Comma-separated. |
| `BVC_ACCESS_TOKEN` | `server.minecraft.access_token` |
| `BVC_TELEMETRY` | `server.features.telemetry`. `true` or `false`. |
| `BVC_CHAT` | `server.features.chat`. `true` or `false`. |
| `BVC_RADAR` | `server.features.radar`. `true` or `false`. |

## Voice

| Variable | Sets |
|---|---|
| `BVC_RECORDING` | `voice.recording.enabled`. `true` or `false`. |
| `BVC_MAX_CONNECTIONS` | `voice.limits.connections`. The maximum number of voice sessions at the same time. `0` means no limit. |
| `BVC_RECONNECT_GRACE` | `voice.limits.reconnect_grace`. The number of seconds that a disconnected player keeps their slot. |

## Runtime

Only the Java mod's embedded server reads these variables. A standalone server ignores them.

| Variable | Sets |
|---|---|
| `BVC_RUNTIME_WORKER_THREADS` | Number of Tokio worker threads. The default is the number of CPU cores on the host. |
| `BVC_RUNTIME_MAX_BLOCKING_THREADS` | Maximum number of threads in the blocking pool. |

## TLS

| Variable | Sets |
|---|---|
| `BVC_TLS_CERTIFICATE` | `server.tls.certificate` |
| `BVC_TLS_KEY` | `server.tls.key` |
| `BVC_TLS_CERTS_PATH` | `server.tls.certs_path` |
| `BVC_TLS_NAMES` | `server.tls.names`. Comma-separated. |
| `BVC_TLS_IPS` | `server.tls.ips`. Comma-separated. |

## Enrollment

In a container, set the enrollment token with these variables. See [assigned hostnames](/wiki/server/enrollment/).

| Variable | Sets |
|---|---|
| `BVC_ENROLLMENT_TOKEN` | `server.enrollment.token` |
| `BVC_ENROLLMENT_ADDRESS` | `server.enrollment.address` |

Use only one certificate source: an enrollment token, ACME, or a TLS certificate and key. If you set more than one, the server does not start.

`BVC_REGISTRY_PEERLINK` has no effect on a running server. It is read only when the binary is compiled.

## ACME

If the config file has no `acme` block, setting any of these variables creates one. A new `acme` block needs both `BVC_ACME_EMAIL` and `BVC_ACME_PROVIDER`. If one is missing, the server does not start, and the error names the missing variable.

| Variable | Sets |
|---|---|
| `BVC_ACME_EMAIL` | Account contact email. |
| `BVC_ACME_PROVIDER` | `cloudflare` or `acme-dns`. |
| `BVC_ACME_API_TOKEN` | Cloudflare API token. |
| `BVC_ACME_DIRECTORY` | ACME directory URL. |
| `BVC_ACME_DOMAINS` | Certificate domains. Comma-separated. |
| `BVC_ACME_DNS_URL` | acme-dns server URL. |
| `BVC_ACME_DNS_USERNAME` | acme-dns username. |
| `BVC_ACME_DNS_PASSWORD` | acme-dns password. |
| `BVC_ACME_DNS_SUBDOMAIN` | acme-dns subdomain. |

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

The server checks the TLS settings at startup. See the [database section](/wiki/reference/configuration/#database) for the combinations that it rejects.

## Bedrock

| Variable | Sets |
|---|---|
| `BVC_BEDROCK_SERVERS` | Replaces the whole server list. |

`BVC_BEDROCK_SERVERS` takes comma-separated `Name@host[:port][@protocol][@mode]` entries:

```
BVC_BEDROCK_SERVERS="My SMP@play.example.com:19132@1001@net,Other@mc.example.net@no_net"
```

`protocol` is a protocol version number. `mode` is `net` or `no_net`. If you omit `mode`, it is `net`. You can omit `protocol`, `mode`, or both, and you can write them in either order. If a value is not a number, `net` or `no_net`, the server does not start, and the error names the value. See [`server.bedrock.servers`](/wiki/reference/configuration/#serverbedrockservers).

Use a hostname. Do not use an IPv6 address. The parser reads its colons as a port separator.

## Meridian

`BVC_MERIDIAN_*` configures registration with Meridian, the proxy for BVC managed hosting. A self-hosted server does not use these variables.

## CLI variables

The CLI reads these variables. The server does not.

| Variable | Used by |
|---|---|
| `BVC_SERVER` | `--server-url` for `login`. The server also reads `BVC_SERVER` (see [Server](#server)). A URL value is for the CLI. A `host:port` value is for the server. |
| `BVC_IDENTITY` | `--identity`, as `<gamertag>:<game>`. |
| `BVC_IDENTITY_DIR` | The directory for stored CLI identities. |

See [CLI](/wiki/reference/cli/).

## Environment interpolation

You can use `${env.VAR}` anywhere in `config.hcl`:

```hcl
minecraft {
    access_token = "${env.BVC_ACCESS_TOKEN}"
}
```

If a referenced variable is not set, the server does not start.
