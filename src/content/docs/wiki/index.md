---
title: Bedrock Voice Chat
description: Proximity voice chat for Minecraft Bedrock and Java — what it is, what the parts are, and where to start.
---

Bedrock Voice Chat (BVC) is a proximity voice-chat system for Minecraft Bedrock and Minecraft Java. Players hear each other based on in-game distance. BVC is cross-platform (Bedrock + Java) and cross-device (PC, mobile, console).

## How Bedrock Voice Chat Works

BVC is composed of 3 separate components that need to be installed.

:::caution
All three components listed are **mandatory**. Bedrock Voice Chat is a system, and omitting one part of the system will _prevent_ BVC from working.
:::

| Part | Who installs it | What it does |
|---|---|---|
| **A Bedrock Dedicated Server or Realms Addon** | Server operator | An Addon (either for BDS or Realms/No-Net servers) or Fabric/Paper mod (Java) that reports player positions. |
| **BVC server** | Server operator | A standalone Rust executable that carries the voice traffic. It runs separately from your Minecraft server. |
| **Client app** | Every player | The BVC app, running on Windows, macOS, Linux, iOS, and Android. It connects you to BVC-supported worlds, lets you hear and talk with others, and manages your settings. |

:::note
_Most_ commercial Minecraft Bedrock hosts _will not_ let you run standalone executables alongside your game server. Expect to install Bedrock Voice Chat server on your own virtual private server, or docker host.
:::

:::tip
The Java mod ships with Bedrock Voice Chat server. Only run the embedded server if your host will not suspend your Java server. It is not suited to low-end hosts, or hosts that suspend inactive instances.
:::

## Where to start

**Playing on a server that already has BVC** — install the app and sign in. [For players](/wiki/player/).

**Adding BVC to your server** — [Running a server](/wiki/server/).

**Checking whether your setup is supported** — Realms, Aternos, Geyser, LAN, consoles. [Where BVC works](/wiki/platforms/).

**Streaming or recording** — [Streaming & recording](/wiki/creator/).

## Setup order

Install in this order. The Addon needs the BVC server's address.

1. [Run the BVC server](/wiki/server/installation/). It needs a reachable host and a TLS certificate, which BVC can obtain itself.
2. Install the Addon, [Bedrock](/wiki/server/bedrock-addon/) or [Java](/wiki/server/java-mod/), pointing it at that address.
3. [Whitelist your players](/wiki/server/players-and-permissions/). BVC is deny-by-default. Nobody can sign in until you add them.
4. Send everyone to [Downloads](/wiki/start/downloads/).
