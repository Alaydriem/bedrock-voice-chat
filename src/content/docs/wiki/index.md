---
title: Bedrock Voice Chat
description: Proximity voice chat for Minecraft Bedrock and Java. Components, setup order, and where to start.
---

Bedrock Voice Chat (BVC) is proximity voice chat for Minecraft Bedrock and Minecraft Java. Players hear each other based on their distance in the game. BVC works with Bedrock and Java, on PC, mobile and console.

## How Bedrock Voice Chat Works

Bedrock Voice Chat has three components. You must install all three.

:::caution
All three components are **mandatory**. If one component is missing, Bedrock Voice Chat does not work.
:::

| Component | Installed by | Purpose |
|---|---|---|
| **Addon or mod** | Server operator | The Bedrock Addon (Bedrock Dedicated Server), the no-net Addon (Realms, Aternos and other hosts that do not allow Addons to use the network), or the Fabric or Paper mod (Java). It sends player positions to the BVC server. |
| **BVC server** | Server operator | A separate program that sends voice between players. It runs separately from your Minecraft server. |
| **Client app** | Every player | The BVC app for Windows, macOS, Linux, iOS and Android. Players use it to join a BVC server and talk to other players. |

:::note
Most commercial Minecraft Bedrock hosts do not let you run other programs next to the game server. Install the Bedrock Voice Chat server on your own virtual private server or Docker host.
:::

:::tip
The Java mod includes a copy of the BVC server that can run inside your Java server (embedded mode). Do not use embedded mode on low-end hosts or on hosts that suspend inactive servers.
:::

## Where to start

| Goal | Go to |
|---|---|
| Play on a server that already has BVC | [For players](/wiki/player/) |
| Add BVC to your server | [Running a server](/wiki/server/) |
| Check support for Realms, Aternos, Geyser, LAN worlds and consoles | [Where BVC works](/wiki/platforms/) |
| Stream or record | [Streaming & recording](/wiki/creator/) |

## Setup order

Install in this order. The Addon needs the address of the BVC server.

1. [Run the BVC server](/wiki/server/installation/). The server needs a host that players can reach, and a TLS certificate. BVC can get the certificate for you.
2. Install the [Bedrock Addon](/wiki/server/bedrock-addon/) or the [Java mod](/wiki/server/java-mod/). Set the BVC server address in its configuration.
3. Let your players in. A player can sign in after they join your game server once, or after you [add them](/wiki/server/players-and-permissions/).
4. Send your players to [Downloads](/wiki/start/downloads/).
