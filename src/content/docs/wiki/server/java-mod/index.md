---
title: Java mod
description: BVC on Fabric and PaperMC, in external or embedded mode.
sidebar:
  label: Overview
  order: 1
---

Bedrock Voice Chat runs on Minecraft Java servers with a mod for Fabric and PaperMC. Download it from [Modrinth](https://modrinth.com/mod/bedrock-voice-chat). For crossplay with Bedrock players, use [Geyser and Floodgate](/wiki/server/java-mod/integrations/geyser-and-floodgate/).

## Loaders

| Loader | Directory | Configuration |
|---|---|---|
| [Fabric](/wiki/server/java-mod/fabric/) | `mods/` | JSON |
| [PaperMC](/wiki/server/java-mod/papermc/) | `plugins/` | YAML |

## Integrations

The mod detects other plugins and mods at startup. See [Integrations](/wiki/server/java-mod/integrations/) for [Geyser and Floodgate](/wiki/server/java-mod/integrations/geyser-and-floodgate/) and [Simple Voice Chat](/wiki/server/java-mod/integrations/simple-voice-chat/).

## External or embedded server

The mod connects to an external BVC server, or runs an embedded BVC server.

**External** mode connects to a standalone BVC server. Use external mode on most hosts. It gives better performance.

**Embedded** mode runs BVC inside the game server's JVM. You do not run a separate service. Use it for small servers and local development. Some hosts, for example [Nodecraft](/wiki/platforms/nodecraft/), support embedded mode.

:::caution
Embedded mode needs a host that runs all the time and has enough memory. On low-end hosts, or hosts that suspend idle servers, audio quality and performance can be bad. These problems are difficult to trace. If you see them, move BVC to its own machine.
:::

## Support

Report problems with the mod on [GitHub](https://github.com/Alaydriem/bedrock-voice-chat/issues).
