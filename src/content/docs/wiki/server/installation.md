---
title: Installing Bedrock Voice Chat Server
description: Stand up the standalone BVC server with Docker, a config file, and automatic TLS.
sidebar:
  label: Installation
  order: 2
---

Bedrock Voice Chat Server is the heart of Bedrock Voice Chat, and is the core component that allows Bedrock and Java players to chat with each other on every platform. Before you install the addon and connct to Bedrock Voice Chat, you'll need to first setup the server.

## Requirements

Before you can run Bedrock Voice Chat Server, there's a few dependencies you'll need to acquire.

1. A linux server, windows computer, or app platform that allows you to run Docker containers
:::tip[These providers work well, and the links support development]
A single core, 512M server can support ~ 5 concurrent players. If you experience hiccups, consider adding more cores, or changing to a dedicated affinity rather than shared.
- [DigitalOcean](https://m.do.co/c/15e066af535c) — $200 credit after $25 spend.
- [Hetzner](https://hetzner.cloud/?ref=StImwLqshuwn) — $25 signup credit.
:::

2. A domain name
:::tip[Recommended Providers]
A few recommended providers are listed here.
- [Namecheap](https://https://www.namecheap.com/)
- [Cloudflare](https://www.cloudflare.com/products/registrar/)
- [TailScale ts.net](https://tailscale.com/)
:::

:::caution[A domain name is REQUIRED for Bedrock Voice Chat to work]
Bedrock Voice Chat does not work with direct IP access. A fully qualified domain name is required.
:::

3. A TLS Certificate

You can either use any valid ACME-DNS provider (Lets Encrypt), or Cloudflare DNS
:::tip[Cloudflare DNS has first class support]
Bedrock Voice Chat can automatically provision a TLS certificate for you if you use an ACME-DNS provider, or Cloudflare. Cloudflare is recommended because it is free, and easy to use and setup if you purchased your domain through Cloudflare or transfered your domain to Cloudflare.
:::

## Firewall Configuration

Your Bedrock Voice Chat server needs the following firewall ports opened to work

| Port | Protocol | Purpose |
|------|----------|---------|
| 443 | tcp | Enables API, Websockets, and fallback voice traffic |
| 443 | udp | Primary voice traffic ingress |
| 28280 | udp | Fallback voice traffic ingress, if 443/udp is blocked |
| 28283 | tcp | Minecraft Server transfer port |

## Running Bedrock Voice Chat with Docker

Once you have your server setup and domain provisioned, setup with BVC can be done in seconds with Docker.

:::note
You can also install and BVC using the natively provided server binaries over on Github, however Docker remains the preferred and recommended way to run BVC Server.
:::

1. Create your configuration file. A minimum example configuration is as shown

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

:::tip[Bedrock Voice Chat is highly configurable!]
Review the full [configuration reference](/wiki/reference/configuration/) to see all the ways you can tune and tailor your
:::

2. Start your server with Docker

```bash
docker run -d --restart=always \
  -p 443:443/tcp -p 443:443/udp -p 28280:28280/udp -p 28283:28283/tcp \
  -e CF_API_TOKEN="your_cloudflare_api_token"
  -v ${PWD}:/data \
  ghcr.io/alaydriem/bedrock-voice-chat/server:beta.21
```

And you're done!

## Then

1. Install the Addon: [Bedrock](/wiki/server/bedrock-addon/) or [Java](/wiki/server/java-mod/).
2. Bedrock Voice Chat requires that you either [whitelist your players](/wiki/server/players-and-permissions/) or have them log into your Bedrock Dedicated Server at least once before using the app.
3. Point players at [Downloads](/wiki/start/downloads/).

Realms, Aternos, and consoles need extra setup. See [Where BVC works](/wiki/platforms/).

## Running BVC in the Java mods

If you're running a Geyser server with PaperMC or Fabric, Bedrock Voice Chat can also be installed there directly. See the [Java installation guide](wiki/server/java-mod/) for more details.