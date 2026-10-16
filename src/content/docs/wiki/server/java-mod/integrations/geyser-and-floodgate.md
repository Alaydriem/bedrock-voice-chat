---
title: Geyser and Floodgate
description: BVC on Geyser and Floodgate cross-platform servers.
sidebar:
  label: Geyser and Floodgate
  order: 2
---

Bedrock Voice Chat supports Geyser servers. Your Bedrock and Java players can hear each other in the same world. Install the [Java mod](/wiki/server/java-mod/) on Fabric or PaperMC, and configure it the same as on any Java server.

:::note
BVC needs no Geyser settings. Geyser shows Bedrock players to the Java server as Java players. The Java mod sends positions for all players in the same way.
:::

## Floodgate

When [Floodgate](https://geysermc.org/wiki/floodgate/) is on the classpath, Bedrock Voice Chat detects it at startup. You do not configure anything.

The mod matches each Floodgate player to their Xbox gamertag. It removes the Floodgate prefix character from the name. Bedrock players sign in to the BVC app with the gamertag that they use in game.

:::caution
Without Floodgate, the mod uses the Java username and logs:

```
Floodgate API not found on classpath — prefix-strip path disabled
```

Bedrock players must then sign in with their Java username, not their gamertag. Install Floodgate.
:::

## Setup

1. Install the Java mod for [Fabric](/wiki/server/java-mod/fabric/) or [PaperMC](/wiki/server/java-mod/papermc/).
2. Install Floodgate with Geyser.

## Server mode

External and embedded mode both work. Use external mode on a Geyser server. Crossplay servers often have more players, and embedded mode uses the same CPU as the game server. See [Java mod](/wiki/server/java-mod/#external-or-embedded-server) for the two modes.

## Players

All players need the BVC app, on Java and on Bedrock. Bedrock players on a console use the mobile app on their own phone or tablet. See [console and mobile](/wiki/platforms/console-and-mobile/).
