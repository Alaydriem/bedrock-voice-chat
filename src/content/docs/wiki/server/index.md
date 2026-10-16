---
title: Running a server
description: What you need to add BVC to your Minecraft server, and the order to do it in.
sidebar:
  label: Overview
  order: 1
---

To add Bedrock Voice Chat to your Minecraft server, run the BVC server and install one Addon or mod.

## BVC server

The BVC server is a standalone Rust program. Run it in Docker, or run the native binary. It is not a Minecraft plugin and does not run inside your game server. Most commercial Minecraft hosts do not let you run it next to the game server. Run it on a VPS, a dedicated server, or a cluster.

On Bedrock, the game server side is an Addon. On Java, it is a Fabric or Paper mod. The Addon or the mod sends player positions to the BVC server over HTTP.

The Java mod can also run BVC inside the game server's JVM. You then do not need a separate BVC service. Use this for small servers that run all the time. Do not use it on low-end hosts or on hosts that suspend idle servers.

## Installation order

Do these steps in order. If you do them out of order, a new installation can appear to do nothing.

1. [Install the BVC server](/wiki/server/installation/). You need a host that clients can reach, and a TLS certificate. BVC can get and renew the certificate itself. See [TLS](/wiki/server/tls/).
2. Install the [Bedrock Addon](/wiki/server/bedrock-addon/) or the [Java mod](/wiki/server/java-mod/). Set it to the address of the BVC server.
3. Let your players in. A player can sign in after they join your game server once, or after you [add them](/wiki/server/players-and-permissions/).
4. Send your players to [Downloads](/wiki/start/downloads/).

## References

| Page | Contents |
|---|---|
| [BDS Addon](/wiki/server/bedrock-addon/) | The pack for Bedrock Dedicated Servers. |
| [No-net Addon](/wiki/server/nonet-addon/) | The pack for Realms, Aternos, and hosts without `@minecraft/server-net`. |
| [Docker](/wiki/server/docker/) | Compose, volumes, health checks, configuration from environment variables. |
| [Monitoring](/wiki/server/monitoring/) | Prometheus metrics, health endpoints, logs. |
| [Troubleshooting](/wiki/server/troubleshooting/) | Problems seen from the operator side. |

Realms, Aternos and consoles need more setup. Start at [Where BVC works](/wiki/platforms/).
