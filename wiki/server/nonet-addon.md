---
title: "No-net Addon"
description: "Build and install the Addon for Realms, Aternos, and hosts without @minecraft/server-net."
source: https://www.bedrockvoicechat.com/wiki/server/nonet-addon/
---
The no-net Addon lets Bedrock Voice Chat run on hosts that do not allow `@minecraft/server-net`, such as Realms and Aternos. Without that module, the standard Addon cannot send HTTP requests. It cannot send positions to the BVC server.

With the no-net Addon, the BVC app runs a local Bedrock server. Minecraft connects to this local server. The app sends all game traffic to the real host, and reads player positions from that traffic.

For host-specific steps, see [Realms](/wiki/platforms/realms/) and [Aternos](/wiki/platforms/aternos/).

## Before you start

Install and configure the [BVC server](/wiki/server/installation/) first.

:::caution
The no-net Addon needs a Bedrock Voice Chat server.
:::

Install the [BVC app](/wiki/start/downloads/). You use the app to connect to the host.

## Limitations

- The Addon is **locked to one protocol version**. An Addon for one Bedrock version does not work with a different version.
- Support for each Minecraft release comes after the release. Mojang does not document the packet protocol and changes it in each update. The BVC developers cannot update the Addon until the Minecraft update is released.
- If your host updates Minecraft automatically, voice chat stops on the day of the update.
- Your host's Minecraft version must match the version that the Addon supports. If the versions do not match, voice chat does not work. Check [version support](/wiki/platforms/version-support/) before you choose a server version.

These limits do not apply to your own Bedrock Dedicated Server with the [full Addon](/wiki/server/bedrock-addon/).

## Build the world

1. Download `no-net.mcaddon` from [GitHub Releases](https://github.com/Alaydriem/bedrock-voice-chat/releases) (releases with a tag that starts with `mods-`), or from CurseForge. Use the `no-net` variant. The full Addon does not work on these hosts.

2. Double-click the file to install it in your Minecraft Bedrock client.

3. Create a new world. Under **Behavior Packs**, enable the **Bedrock Voice Chat (Realms + No-Net Servers)** pack.

   <div class="window"><img src="/assets/wiki/mc-packs-active.jpg" alt="Pack list with the Bedrock Voice Chat (Realms + No-Net Servers) pack active" /></div>

4. Under **Experiments**, enable **Beta APIs**.

   <div class="window"><img src="/assets/wiki/mc-experiments.jpg" alt="Experiments toggle showing Beta APIs enabled" /></div>

5. Create the world, then exit it immediately.

6. Open the world's settings and export the world. Save the `.mcworld` file where you can find it again.

   <div class="window"><img src="/assets/wiki/mc-export-world.jpg" alt="World settings with the Export World option at the bottom" /></div>

## Upload and connect

Upload the world to your host. Then connect the BVC app to the host with Bedrock Voice Chat Connect. The upload step is different for each host:

- **[Realms](/wiki/platforms/realms/)**: upload the world to the Realm.
- **[Aternos and other no-net hosts](/wiki/platforms/aternos/)**: rename the file to `.zip`, then upload it.

## Console players

Console players need no extra steps. The BVC app shows the world on your local network, and Minecraft lists it under **Worlds**. See [console and mobile](/wiki/platforms/console-and-mobile/).

## Chat

[Chat](/wiki/player/chat/) needs no configuration on a no-net host. The BVC app reads chat from the game traffic, in the same way as positions. The app can read and send in-game chat after you connect.

The `bvc_world_name` variable of the [full Addon](/wiki/server/bedrock-addon/) does not apply to the no-net Addon.