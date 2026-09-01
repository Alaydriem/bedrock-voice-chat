---
title: No-net Addon
description: Build and install the Addon for Realms, Aternos, and hosts without @minecraft/server-net.
sidebar:
  label: No-net Addon
  order: 7
---

The no-net Addon lets Bedrock Voice Chat run on hosts that do not permit `@minecraft/server-net`, such as Realms and Aternos. Without that module the standard Addon cannot make HTTP requests, and cannot report positions the normal way.

The no-net Addon works differently. Your BVC client becomes the Bedrock server your game connects to, proxies packets through to the real host, and reads positions off the wire.

The steps below are the same for every such host. Host-specific steps are in [Realms](/wiki/platforms/realms/) and [Aternos](/wiki/platforms/aternos/).

## Before you start

Install and configure the [BVC server](/wiki/server/installation/) first.

:::caution
Bedrock Voice Chat Server is _not_ an optional component. The no-net path still needs this.
:::

Install the [client app](/wiki/start/downloads/). Part of this happens there.

## Read the limitations first

Bedrock's Addon system constrains this approach:

- The Addon is **locked to a protocol version**. One built for a given Bedrock version will not work against another.
- Support **lags every Minecraft release**. Mojang does not document the packet protocol and changes it each update. The work cannot start until the update ships. There is no day-one support.
- A host that auto-updates Minecraft **will break voice** on update day.

Running your own Bedrock Dedicated Server avoids all of it. See [version support](/wiki/platforms/version-support/).

## Build the world

1. Download the `no-net.mcaddon` package from GitHub Releases under the `mods-` tag, or from CurseForge. It must be the `no-net` variant. The full Addon does not work here.

2. Double-click it to install into your local Minecraft Bedrock client.

3. Create a new world. Under **Behavior Packs**, enable the Bedrock Voice Chat Realms + No-Net pack.

   <div class="shot"><span>Behavior Packs list with the Realms + No-Net pack enabled</span></div>

4. Under **Experiments**, enable **Beta APIs**.

   <div class="shot"><span>Experiments toggle showing Beta APIs enabled</span></div>

5. Create the world, then exit it immediately.

6. Open the world's settings and export it.

   <div class="shot"><span>World settings with the Export World option at the bottom</span></div>

Save it somewhere you can find.

## Chat

[Chat](/wiki/player/chat/) needs no configuration on a no-net host. The proxy session reads chat off the wire the same way it reads positions. The app can read and send in-game chat as soon as you connect.

The `bvc_world_name` variable on the [full Addon](/wiki/server/bedrock-addon/) does not apply here.

## Then

Upload the world to your host, then point the client at it with Bedrock Voice Chat Connect. The upload step differs per host:

- **[Realms](/wiki/platforms/realms/)** — upload the world to the Realm.
- **[Aternos and other no-net hosts](/wiki/platforms/aternos/)** — rename to `.zip`, then upload.

## Match the versions

Your host's Minecraft version must match the one the Addon supports. These Addons are not backwards compatible. A mismatch fails outright.

Check [version support](/wiki/platforms/version-support/) before picking a server version.

## Console players

Nothing extra. The app hosts the session on your network, and it appears in Minecraft under **Worlds**. See [console and mobile](/wiki/platforms/console-and-mobile/).
