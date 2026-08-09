---
title: Where BVC works
description: Compatibility across Bedrock Dedicated Server, Java, Realms, Aternos, Geyser, consoles, and LAN.
sidebar:
  label: Compatibility
  order: 1
---

BVC needs a server it can install an Addon onto. Real servers work. Worlds hosted by the Minecraft game client do not.

## Compatibility

| Setup | Works? | What it takes |
|---|---|---|
| **Bedrock Dedicated Server** | Yes | The [Addon](/wiki/server/bedrock-addon/). This is the path everything else is measured against. |
| **Java — Fabric or Paper** | Yes | The [Java mod](/wiki/server/java-mod/), external or embedded. |
| **Geyser / Floodgate** | Yes | The Java mod. Floodgate is auto-detected; nothing BVC-specific to configure. [Details](/wiki/platforms/geyser/). |
| **Realms** | Yes | The [no-net Addon](/wiki/server/nonet-addon/) plus Realms Connect in the app. [Details](/wiki/platforms/realms/). |
| **Aternos and other no-net hosts** | Yes | The no-net Addon plus Proxy Connect. [Details](/wiki/platforms/aternos/). |
| **Consoles** | Yes | The mobile app. On Realms or Aternos it also needs the server's DNS responder enabled. [Details](/wiki/platforms/console-and-mobile/). |
| **LAN worlds** | No | Nothing to install an Addon into. [Why](/wiki/platforms/lan-and-local-worlds/). |
| **Locally hosted / single-player worlds** | No | Same reason. [Why](/wiki/platforms/lan-and-local-worlds/). |

## Bedrock Dedicated Server vs Realms

BVC needs a server-side Addon to report where players are. A Bedrock Dedicated Server, a Java server, a Realm, and an Aternos instance are all servers you can upload a world and its Addon to.

A LAN world or single-player world is hosted by the game client, which cannot load server-side Addons. There is nowhere to put one, and BVC never receives position data.

## Realms and Aternos

Both speak Bedrock's own network protocol, which Mojang does not document and changes every release. The consequences:

- Support **lags each Minecraft update**, sometimes by a while. There is no guaranteed day-one support.
- The Addon is **locked to a protocol version** — server and pack must match.
- An auto-updating host **will break voice** on update day.

See [version support](/wiki/platforms/version-support/) for what is currently supported and why.
