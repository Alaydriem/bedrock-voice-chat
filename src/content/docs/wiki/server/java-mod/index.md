---
title: Java mod
description: BVC on Fabric and PaperMC, in external or embedded mode.
sidebar:
  label: Overview
  order: 1
---

Bedrock Voice Chat runs on Minecraft Java servers through a mod published on [Modrinth](https://modrinth.com/mod/bedrock-voice-chat) for both Fabric and PaperMC with crossplay through [Geyser and Floodgate](/wiki/server/java-mod/integrations/geyser-and-floodgate/).

## Pick your loader

- **[Fabric](/wiki/server/java-mod/fabric/)** — `mods/` directory, JSON config.
- **[PaperMC](/wiki/server/java-mod/papermc/)** — `plugins/` directory, YAML config.

## Integrations

The mod detects other plugins and mods at startup. See [Integrations](/wiki/server/java-mod/integrations/) for [Geyser and Floodgate](/wiki/server/java-mod/integrations/geyser-and-floodgate/) and [Simple Voice Chat](/wiki/server/java-mod/integrations/simple-voice-chat/).

## Integrated or External BVC Server?

Bedrock Voice Chat can run either with an embedded voice chat server, or an external one. For most Minecraft hosts, you'll want to use an **external** server for performance, however certain Minecraft hosts such as [Nodecraft](/wiki/platforms/nodecraft/) support the embedded server.

**External** points at a standalone BVC server. This should _always_ be your go-to default choice.

**Embedded** runs BVC inside the game server's JVM. No separate service. Suitable for small servers and local development.

Embedded needs an always-on host with adequate RAM. Low-end hosts, and anything that suspends when idle, produce audio quality and performance problems that are hard to attribute. Move BVC to its own machine if you hit them.

## Support

Issues with the mod go to [GitHub](https://github.com/Alaydriem/bedrock-voice-chat/issues).
