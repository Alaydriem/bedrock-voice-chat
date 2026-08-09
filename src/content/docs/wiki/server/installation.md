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

BVC is CPU-bound. Add cores before memory. Two vCPUs handles most communities.

These providers work well, and the links support development:

- [DigitalOcean](https://m.do.co/c/15e066af535c) — $200 credit after $25 spend.
- [Hetzner](https://hetzner.cloud/?ref=StImwLqshuwn) — $25 signup credit.

## Run it with Docker

```bash
docker run -d --restart=always \
  -p 443:443/tcp -p 443:443/udp, -p 8443:8443/udp \
  -v ${PWD}:/data \
  ghcr.io/alaydriem/bedrock-voice-chat/server:<version>
```

Publish both protocols on 443. TCP carries the HTTP API and login. UDP carries the QUIC voice transport. 8443 is a redundant QUIC port for networks that block QUIC over 443.

Mount your `config.hcl` and certificates into `/data`.

`/health/liveness` and `/health/readiness` are for container orchestration. `/api/ping` is a cheap reachability probe. See [Monitoring](/wiki/server/monitoring/).

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

Nothing else is required. Every other key has a working default. The full surface is in the [configuration reference](/wiki/reference/configuration/).

> **`access_token`** is a shared secret. The same value goes into your Addon or mod, and the game server sends it on every position update. Pick something long and random. Left unset, BVC generates one and writes it to the static directory.

`acme` makes BVC obtain and renew its own certificate over DNS-01. You never touch PEM files, and port 80 stays closed. Full setup, including acme-dns and bring-your-own-certificate, is in [TLS certificates](/wiki/server/tls/).

### Secrets belong in the environment

`${env.VAR}` is evaluated anywhere in the file. An unset referenced variable is a hard startup error. A missing token stops the server instead of booting it unsecured.

Every setting also has a `BVC_*` environment override, which makes the file optional. With no `config.hcl` the server starts from defaults plus the environment. This is the usual arrangement under Kubernetes. See [environment variables](/wiki/reference/environment-variables/).

## Verify

```bash
curl https://example.bedrockvc.stream/api/config
```

## Upgrading

Pull the new image and restart. Database migrations run at startup.

**Server and client must speak the same protocol version.** BVC compares major and minor. Patch releases do not change the wire format. A client on a different major or minor is refused, and told which side is behind.

Protocol went from **2.1.0 to 3.0.0** in this release. Every client must update. Schedule it for a time when you can tell your players.

Two other things changed shape in the same release:

- The Java mod's `embedded-config` no longer takes flat keys. See [Migration](/wiki/server/java-mod/#migration).
- The recording format version changed, which makes **existing recordings no longer exportable**. Tell anyone who records to export what they want before updating the app. See [recording](/wiki/creator/recording/#format-version).

The version each side runs is in the client's About pane and in `/api/config`.

## Then

1. Install the Addon: [Bedrock](/wiki/server/bedrock-addon/) or [Java](/wiki/server/java-mod/). Give it this server's URL and the same `access_token`.
2. [Whitelist your players](/wiki/server/players-and-permissions/). BVC is deny-by-default.
3. Point players at [Downloads](/wiki/start/downloads/).

Realms, Aternos, and consoles need extra setup. See [Where BVC works](/wiki/platforms/).
