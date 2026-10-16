---
title: Enabling Beta APIs
description: Turning on the Beta APIs experiment for a Bedrock server world.
sidebar:
  label: Enabling Beta APIs
  order: 8
---

The Bedrock Voice Chat Addon uses beta script modules. You **MUST** enable the **Beta APIs** experiment on the world that runs the Addon.

:::danger
Without Beta APIs, the Addon loads but does nothing. It shows no error and no console line, and it sends no positions.
:::

:::caution[No server.properties setting]
Each world stores its experiments in its own `level.dat` file. A server configuration file cannot enable them. A new world that BDS creates at first start does not have them.
:::

## Create the world in the client

Enable the experiment in the Minecraft client, then move the world to the server.

1. In the Minecraft Bedrock **client**, create a new world.
2. Open **Settings → Experiments** and turn on **Beta APIs**.
3. Add the BVC Addon to the world's behavior packs and resource packs.
4. Create the world, then leave it immediately.
5. Open **Settings**, then select **Export World**. Save the `.mcworld` file.
6. Put the world on your server:
   - **BDS**: rename the file to `.zip`. Extract it into `worlds/<your world name>/`. Set `level-name` in `server.properties` to the same name.
   - **A host with a control panel**: upload the `.mcworld` or `.zip` file with the host's world upload feature.

The [no-net Addon](/wiki/server/nonet-addon/) guide uses the same steps. Realms and Aternos worlds also need Beta APIs.

## Existing worlds

Server settings cannot enable the experiment on an existing world. The setting is in the world's `level.dat`.

There are three methods. The first has the lowest risk.

**Open the world in the client.** Copy the world from the server and open it in the Bedrock client. Enable Beta APIs under Experiments, exit, and copy the world back.

**Use your host's control panel.** Aternos and some other hosts have experiment switches in their control panel. If your host has one, use it.

**Edit `level.dat` with an NBT editor.** The experiment flags are in an `experiments` compound. Set the flag for Beta APIs. Also set the flag that marks the world as a world that uses experiments.

:::caution
The NBT key names change between Bedrock versions. Do not copy a key name from a guide. Create a test world in the client with Beta APIs enabled. Open both `level.dat` files side by side, and copy the differences.
:::

Back up the world before you edit it. A world with a damaged `level.dat` does not load.

## Side effects

**Achievements are permanently disabled** for that world. This applies to every experiment in Minecraft.

**The world is permanently marked as a world that used experiments.** If you turn the experiment off later, the mark stays.

**Experimental features can change between Minecraft versions.**

## Verification

Set this in `server.properties`:

```properties
content-log-console-output-enabled=true
```

When the world loads, this line shows in the console:

```
[BVC] Connecting to: <your server>
```

If the line does not show, the Addon is not running. Check these causes, most likely first:

1. Beta APIs are not enabled on **the world that the server loads**. Check that `level-name` is the world you edited.
2. The Addon is not in the world's pack list.
3. A module is missing from `config/default/permissions.json`. See [Bedrock Addon](/wiki/server/bedrock-addon/).
4. `bvc_server` is not set in `variables.json`.

## Java servers

Java servers do not need Beta APIs. Experiments exist only in Bedrock. See [Java mod](/wiki/server/java-mod/).
