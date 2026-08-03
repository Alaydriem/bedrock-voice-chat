---
title: Java mod
description: BVC on Fabric and PaperMC, in external or embedded mode.
sidebar:
  label: Overview
  order: 1
---

BVC runs on Java servers through a mod published on [Modrinth](https://modrinth.com/mod/bedrock-voice-chat) for both Fabric and PaperMC. Geyser servers work too — see [Geyser](/wiki/platforms/geyser/).

## Pick your server

- **[Fabric](/wiki/server/java-mod/fabric/)** — `mods/` directory, JSON config.
- **[PaperMC](/wiki/server/java-mod/papermc/)** — `plugins/` directory, YAML config.

The settings are the same on both; only the file format and location differ.

## Pick your mode

**External** points at a standalone BVC server. Use it for production, for more than about 15 players, or when one BVC server serves several game servers.

**Embedded** runs BVC inside the game server's JVM, so there is no separate service. Convenient for small servers and local development.

Embedded needs an always-on host with adequate RAM. On low-end hosts, or anything that suspends when idle, you get audio quality and performance problems that are hard to attribute. If you hit them, move BVC to its own machine rather than tuning around it.

## Shared settings

| Key | Default | Description |
|---|---|---|
| `bvc-server` | — | URL of the BVC server. Required in external mode. |
| `access-token` | — | Must match `server.minecraft.access_token` on the BVC server. Required in external mode. |
| `minimum-players` | `1` | Players online before positions are relayed. |
| `use-embedded-server` | `false` | Run BVC inside this JVM. |
| `embedded-config` | — | Required when `use-embedded-server` is true. |

Keys accept both `hyphen-case` and `camelCase`.

### Embedded settings

| Key | Default | Description |
|---|---|---|
| `http-port` | `8444` | HTTP/REST and login. |
| `quic-port` | `8443` | UDP/QUIC voice. |
| `broadcast-range` | `32.0` | Max distance in blocks at which audio is relayed. |
| `tls-certificate` | `""` | Fullchain TLS PEM from a public CA. |
| `tls-key` | `""` | TLS private key PEM. |
| `tls-names` | `["localhost", "127.0.0.1"]` | SAN hostnames clients connect to. |
| `tls-ips` | `["127.0.0.1"]` | SAN IPs clients connect to. |
| `log-level` | `"info"` | `trace`, `debug`, `info`, `warn`, `error`. |
| `assets-path` | — | Override the assets directory. |
| `allow-audio-upload` | `false` | Permit clients with `audio_upload`. |
| `allow-audio-delete` | `false` | Permit clients with `audio_delete`. |

Note the ports: embedded mode defaults to **8444/8443**, not 443 like the standalone
server. `broadcast-range` also defaults to 32 here, against 48 standalone.

### Embedded Bedrock relay

Set under `bedrock` inside `embedded-config`. Off by default.

| Key | Default | Description |
|---|---|---|
| `enabled` | `false` | Bedrock relay for Proxy Connect and Realms Connect. |
| `transfer-port` | `19139` | Port players point Minecraft at. |
| `transfer-target-port` | `19137` | Internal handoff port. |
| `transfer-cache-ttl-secs` | `900` | How long a resolved transfer target is cached. |

### Certificates in embedded mode

Two systems, one of which is yours:

| | Purpose | Managed by |
|---|---|---|
| HTTPS TLS | Clients trusting the server | You — `tls-certificate` and `tls-key`, from a public CA |
| QUIC mTLS CA | Server authenticating clients | The mod, automatically |

The mTLS CA is generated into the mod's data directory: `config/bedrock-voice-chat/` on Fabric, `plugins/BedrockVoiceChat/` on Paper. Leave it alone, and back it up — deleting it invalidates every client certificate already issued.

Embedded mode has no ACME support. External mode does, and it is the easier path if certificate management is what is stopping you. See [TLS](/wiki/server/tls/).

## Support

Issues with the mod go to [GitHub](https://github.com/Alaydriem/bedrock-voice-chat/issues).
