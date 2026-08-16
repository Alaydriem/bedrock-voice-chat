---
title: Geyser
description: BVC on Geyser and Floodgate cross-platform servers.
sidebar:
  label: Geyser
  order: 5
---

Bedrock Voice Chat supports Geyser servers, letting your Bedrock and Java players hear each other in the same world. Install the [Java mod](/wiki/server/java-mod/) on Fabric or PaperMC and configure it as you would on any Java server.

:::note
Nothing here is Geyser-specific. Geyser presents Bedrock players to a Java server, and the Java mod reports positions the same way for all of them.
:::

## Floodgate

When [Floodgate](https://geysermc.org/wiki/floodgate/) is on the classpath, Bedrock Voice Chat detects it at runtime. There is nothing to configure.

Bedrock players authenticated through Floodgate are matched to their raw Xbox gamertag. Floodgate's prefix character is stripped automatically. They sign into the BVC app with the same gamertag they use in game.

:::caution
Without Floodgate, the mod falls back to the Java-side username and logs:

```
Floodgate API not found on classpath — prefix-strip path disabled
```

Your Bedrock players then need to know their Java username to sign in, which is **not** what they expect. Install Floodgate.
:::

## Setup

1. Install the [Java mod](/wiki/server/java-mod/) — [Fabric](/wiki/server/java-mod/fabric/) or [PaperMC](/wiki/server/java-mod/papermc/).
2. Have Floodgate installed alongside Geyser, which is the standard cross-play setup anyway.
3. Nothing else.

## Which mode

Both external and embedded work. Use external on a Geyser server. Cross-play servers tend to have more players, and embedded mode competes with the game server for CPU. See [Java mod](/wiki/server/java-mod/) for the tradeoff.

## Players

Everyone needs the BVC app, Java and Bedrock alike. Bedrock players on console use the mobile app on their personal device. See [console and mobile](/wiki/platforms/console-and-mobile/).
