---
title: Installing Bedrock Voice Chat Server
description: Install the standalone BVC server with Docker, a configuration file, and automatic TLS.
sidebar:
  label: Installation
  order: 2
---

Bedrock Voice Chat Server is the central part of Bedrock Voice Chat. Bedrock and Java players connect to it to talk to each other. Set up the server before you install the Addon.

## Requirements

1. A Linux server, a Windows computer, or a hosting platform that runs Docker containers.
2. A domain name.
3. A TLS certificate.

:::tip[Hosting providers]
A server with 1 CPU core and 512 MB of memory supports about 5 players at the same time. If audio stutters, add CPU cores, or change to a plan with dedicated CPU cores.

These links support development:

- [DigitalOcean](https://m.do.co/c/15e066af535c): referral credit after you spend $25.
- [Hetzner](https://hetzner.cloud/?ref=StImwLqshuwn): $25 credit when you sign up.
:::

:::tip[Domain registrars]
- [Namecheap](https://www.namecheap.com/)
- [Cloudflare](https://www.cloudflare.com/products/registrar/)
- [Tailscale](https://tailscale.com/) (`ts.net` names)
:::

:::caution[A domain name is REQUIRED]
Bedrock Voice Chat does not work with a bare IP address. You **MUST** use a fully qualified domain name.
:::

BVC gets and renews its TLS certificate through ACME DNS-01 with Let's Encrypt. It supports two DNS providers: Cloudflare and acme-dns. You can also use your own certificate files. See [TLS certificates](/wiki/server/tls/).

:::tip[Cloudflare DNS]
Use Cloudflare if your domain is registered with Cloudflare or moved to Cloudflare. Cloudflare DNS is free.
:::

## Firewall configuration

Open these ports in the firewall.

| Port | Protocol | Purpose |
|------|----------|---------|
| 443 | TCP | HTTP API, WebSocket, and WebSocket voice when UDP is blocked. |
| 28280 | UDP | Voice traffic (QUIC). |
| 28284 | UDP | Peering with other BVC servers. Open it only if you use [peering](/wiki/reference/peering/). |

## Running Bedrock Voice Chat with Docker

Docker is the recommended method. You can also run the native server binary from [GitHub](https://github.com/Alaydriem/bedrock-voice-chat/releases).

1. Create `config.hcl`. This is the minimum configuration:

   ```hcl
   server {
       tls {
           names = ["replace.with.your.domain.name.tld"]

           acme {
               email    = "you@example.com"
               provider = "cloudflare"
               api_token = "${env.CF_API_TOKEN}"
           }
       }
   }
   ```

   All configuration keys are in the [configuration reference](/wiki/reference/configuration/).

2. Start the server with Docker:

   ```bash
   docker run -d --restart=always --name bvc \
     -p 443:443/tcp -p 28280:28280/udp -p 28284:28284/udp \
     -e CF_API_TOKEN="your_cloudflare_api_token" \
     -v ${PWD}:/data \
     ghcr.io/alaydriem/bedrock-voice-chat/server:1.0.0-beta.21
   ```

3. Get your access token:

   ```bash
   docker exec -it bvc bvc-server admin token mint --local -c /data/config.hcl
   ```

   :::caution[Save this token]
   The Bedrock Addon and the Java mod need this token.
   :::

4. Make yourself the first admin. The command creates your player record if it does not exist.

   ```bash
   docker exec -it bvc bvc-server admin bootstrap -p <YourGamertag>
   ```

5. Sign in to the Bedrock Voice Chat app.

## Next steps

1. Install the [Bedrock Addon](/wiki/server/bedrock-addon/) or the [Java mod](/wiki/server/java-mod/).
2. Let your players in. A player can sign in after they join your game server once, or after you [add them](/wiki/server/players-and-permissions/).
3. Send your players to [Downloads](/wiki/start/downloads/).

Realms, Aternos and consoles need more setup. See [Where BVC works](/wiki/platforms/).

## Embedded server in the Java mod

If you run a Geyser server with PaperMC or Fabric, the Java mod can run the BVC server inside the game server. See the [Java mod guide](/wiki/server/java-mod/).
