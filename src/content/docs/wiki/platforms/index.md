---
title: Where BVC works
description: Compatibility across Bedrock Dedicated Server, Java, Realms, Aternos, Geyser, consoles, and LAN.
sidebar:
  label: Compatibility
  order: 1
---

Bedrock Voice Chat runs on any Minecraft server you can install an Addon or mod onto. Below is every supported setup, and what each one needs.

:::caution
Worlds hosted by the Minecraft game client are **not** supported. That covers LAN worlds, locally hosted worlds, and single-player worlds. See [LAN and local worlds](/wiki/platforms/lan-and-local-worlds/).
:::

## Compatibility

| Setup | Works? | What it takes |
|---|---|---|
| **Bedrock Dedicated Server** | Yes | The [Addon](/wiki/server/bedrock-addon/). This is the path everything else is measured against. |
| **Java — Fabric or Paper** | Yes | The [Java mod](/wiki/server/java-mod/), external or embedded. |
| **Geyser / Floodgate** | Yes | The Java mod. Floodgate is auto-detected; nothing BVC-specific to configure. [Details](/wiki/server/java-mod/integrations/geyser-and-floodgate/). |
| **Realms** | Yes | The [no-net Addon](/wiki/server/nonet-addon/) plus Bedrock Voice Chat Connect in the app. [Details](/wiki/platforms/realms/). |
| **Aternos and other no-net hosts** | Yes | The no-net Addon plus Bedrock Voice Chat Connect. [Details](/wiki/platforms/aternos/). |
| **Consoles** | Yes | The mobile app on the same network. The session appears under Worlds. [Details](/wiki/platforms/console-and-mobile/). |
| **LAN worlds** | No | Nothing to install an Addon into. [Why](/wiki/platforms/lan-and-local-worlds/). |
| **Locally hosted / single-player worlds** | No | Same reason. [Why](/wiki/platforms/lan-and-local-worlds/). |

---

See [version support](/wiki/platforms/version-support/) for what is currently supported and why.
