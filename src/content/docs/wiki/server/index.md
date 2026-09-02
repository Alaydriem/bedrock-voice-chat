---
title: Running a server
description: What you need to add BVC to your Minecraft server, and the order to do it in.
sidebar:
  label: Overview
  order: 1
---

Adding Bedrock Voice Chat to your server means running one extra service and installing one Addon. The order you do them in matters more than either.

## What is Bedrock Voice Chat Server?

The BVC server is a standalone Rust executable, run in a Docker container. It is not a Minecraft plugin and does not run inside your game server. Most commercial Minecraft hosts will not let you run it alongside one. Plan on a VPS, a dedicated box, or a cluster.

On Bedrock the server side is an Addon. On Java it is a Fabric or Paper mod. Either way it reports player positions to the BVC server over HTTP.

The Java mod can also run BVC embedded in the game server's JVM, which removes the separate service. That suits small always-on servers. It does not suit low-end hosts or anything that suspends when idle.

## Installation Order

Build from the bottom up. The Addon needs the BVC server's address, and running these out of order is the most common reason a fresh install appears to do nothing.

1. **[Install the BVC server](/wiki/server/installation/)** — needs a reachable host and a certificate. BVC can obtain and renew the certificate itself; see [TLS](/wiki/server/tls/).
2. **Install the Addon** — [Bedrock](/wiki/server/bedrock-addon/) or [Java](/wiki/server/java-mod/) — pointed at that address.
3. **[Whitelist your players](/wiki/server/players-and-permissions/)** — BVC is deny-by-default and nobody can sign in until you add them.
4. Send everyone to [Downloads](/wiki/start/downloads/).

## References

| Page | What it covers |
|---|---|
| [BDS Addon](/wiki/server/bedrock-addon/) | The pack for Bedrock Dedicated Servers |
| [No-net Addon](/wiki/server/nonet-addon/) | The pack for Realms, Aternos, and hosts without `@minecraft/server-net`. |
| [Docker](/wiki/server/docker/) | Compose, volumes, health checks, env-driven config. |
| [Monitoring](/wiki/server/monitoring/) | Prometheus metrics, health endpoints, logs. |
| [Troubleshooting](/wiki/server/troubleshooting/) | Diagnosing it from the operator side. |

Realms, Aternos, and consoles take extra setup on top of this. Start at [Where BVC works](/wiki/platforms/).
