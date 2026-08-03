---
title: Installing the BVC server
description: Stand up the standalone BVC server with Docker, a config file, and automatic TLS.
sidebar:
  label: Installation
  order: 2
---

The BVC server coordinates audio, player state, and positioning for every connected client. It needs a publicly reachable Linux or Windows host and a valid TLS certificate.

Do this before installing the Addon. The Addon needs this server's address.

## Sizing

BVC is CPU-bound. Add cores before memory. Two vCPUs is a reasonable start and should handle most communities without issue.

These providers work well and the links support development:

- [DigitalOcean](https://m.do.co/c/15e066af535c) — $200 credit after $25 spend.
- [Hetzner](https://hetzner.cloud/?ref=StImwLqshuwn) — $25 signup credit.

## Run it with Docker

```bash
docker run -d --restart=always \
  -p 443:443/tcp -p 443:443/udp, -p 8443:8443/udp \
  -v ${PWD}:/data \
  ghcr.io/alaydriem/bedrock-voice-chat/server:<version>
```

Both protocols on 443 are required: TCP carries the HTTP API and login, UDP carries the QUIC voice transport. 8443 serves as a redundant QUIC port for networks that block QUIC over :443.

Mount your `config.hcl` and certificates into `/data`.

`/health/liveness` and `/health/readiness` are there for container orchestration. `/api/ping` is a cheap reachability probe. See [Monitoring](/wiki/server/monitoring/).

## Minimum configuration

Create `config.hcl` next to the server:

```hcl
server {
    tls {
        names = ["example.bedrockvc.stream"]

        acme {
            email    = "you@example.com"
            provider = "cloudflare"
            api_token = "${env.CF_API_TOKEN}"
        }
    }

    minecraft {
        access_token = "${env.BVC_ACCESS_TOKEN}"
    }
}
```

That is all that is required. Everything else has a working default — the full surface is in the [configuration reference](/wiki/reference/configuration/).

> **`access_token`** is a shared secret. The same value goes into your Addon or mod, and the game server sends it on every position update. Pick something long and random. If unset, BVC will generate one for you and place it in the static directory

`acme` makes BVC obtain and renew its own certificate over DNS-01, so you never touch PEM files and port 80 stays closed. Full setup, including acme-dns and bring-your-own-certificate, is in [TLS certificates](/wiki/server/tls/).

### Secrets belong in the environment

`${env.VAR}` is evaluated anywhere in the file. A referenced variable that is unset is a hard startup error, not an empty string — a missing token stops the server instead of booting it unsecured.

Every setting also has a `BVC_*` environment override, so the file is optional. If `config.hcl` is absent the server starts from defaults plus the environment, which is how it is usually run under Kubernetes. See [environment variables](/wiki/reference/environment-variables/).

## Confirm it is up

```bash
curl https://example.bedrockvc.stream/api/config
```

## Then

1. Install the Addon: [Bedrock](/wiki/server/bedrock-addon/) or [Java](/wiki/server/java-mod/). Give it this server's URL and the same `access_token`.
2. [Whitelist your players](/wiki/server/players-and-permissions/) — BVC is deny-by-default and nobody can sign in until you do.
3. Point players at [Downloads](/wiki/start/downloads/).

Supporting Realms, Aternos, or consoles needs extra setup on top of this — see [Where BVC works](/wiki/platforms/).
