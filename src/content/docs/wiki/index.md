---
title: Bedrock Voice Chat
description: Proximity voice chat for Minecraft Bedrock and Java — what it is, what the parts are, and where to start.
---

Bedrock Voice Chat is proximity voice chat for Minecraft Bedrock and Java. Players hear each other based on where they are in the world. It works across platforms and across devices, consoles included.

## The three parts

BVC is not a single download. Three pieces have to be in place, usually installed by different people.

| Part | Who installs it | What it does |
|---|---|---|
| **Addon** | Server operator | An Addon (Bedrock) or Fabric/Paper mod (Java) that reports player positions. |
| **BVC server** | Server operator | A standalone Rust executable that carries the voice traffic. Runs separately from your Minecraft server. |
| **Client app** | Every player | Windows, macOS, Linux, iOS, Android. Connects straight to the BVC server. |

The BVC server is a standalone executable, not a plugin. Most commercial Minecraft hosts will not let you run it beside your game server, so expect to run it on your own VPS, dedicated box, or cluster.

The Java mod is the exception. It can run BVC embedded in the game server's JVM, which suits small always-on servers and does not suit low-end or suspend-prone hosts.

## Where to start

**Playing on a server that already has BVC** — install the app and sign in. [For players](/wiki/player/).

**Adding BVC to your server** — [Running a server](/wiki/server/).

**Checking whether your setup is supported** — Realms, Aternos, Geyser, LAN, consoles. [Where BVC works](/wiki/platforms/).

**Streaming or recording** — [Streaming & recording](/wiki/creator/).

## Setup order

The Addon needs the BVC server's address, so the server has to exist first. Running these out of order is the most common reason a first install appears to do nothing.

1. [Run the BVC server](/wiki/server/installation/). It needs a reachable host and a TLS certificate, which BVC can obtain itself.
2. Install the Addon — [Bedrock](/wiki/server/bedrock-addon/) or [Java](/wiki/server/java-mod/) — pointing it at that address.
3. [Whitelist your players](/wiki/server/players-and-permissions/). BVC is deny-by-default; nobody can sign in until you add them.
4. Send everyone to [Downloads](/wiki/start/downloads/).
