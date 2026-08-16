---
title: Aternos and no-net hosts
description: Run BVC on Aternos and other hosts that cannot enable @minecraft/server-net.
sidebar:
  label: Aternos
  order: 4
---

Bedrock Voice Chat supports Aternos and similar free hosts through the no-net Addon and Bedrock Voice Chat Connect, letting you run proximity voice chat on a host that does not permit `@minecraft/server-net`. Supported as of 1.0.0-beta.15.

Your client becomes the Bedrock server your game connects to, proxies through to the real host, and reads positions off the wire.

:::caution
Support lags every Minecraft release. A host that auto-updates **will break** voice on update day. Check [version support](/wiki/platforms/version-support/) before committing your community to Aternos.
:::

## Before you start

1. Install and configure the [BVC server](/wiki/server/installation/). Not optional.
2. Install the [client app](/wiki/start/downloads/).
3. Build the no-net world following the [no-net Addon](/wiki/server/nonet-addon/) guide.

## Upload the world

1. Find the `.mcworld` you exported and change the extension to `.zip`.
2. Upload it to Aternos.

   <div class="shot"><span>Aternos world upload page</span></div>

3. Set the Aternos server version to match the Addon's supported Minecraft version. These Addons are not backwards compatible — a mismatch fails outright.

   <div class="shot"><span>Aternos server version selector</span></div>

Check [version support](/wiki/platforms/version-support/) for what to pick.

## Connect

1. Sign into the BVC client against the server from step 1.
2. Settings cog, bottom left. Then **Voice Chat Connect** in the sidebar. First visit asks you to sign in again.
3. Add a server with the button at the top: your Aternos domain and port.

   <div class="shot"><span>Bedrock Voice Chat Connect add-server dialog with an Aternos address</span></div>

4. Connect. You get connection details back.

   <div class="shot"><span>Bedrock Voice Chat Connect showing connection information for a connected server</span></div>

## Open Minecraft

The app hosts the session on your network. It appears in Minecraft under **Worlds**.

1. Open Minecraft on the same network as the device running BVC.
2. Go to **Worlds**.
3. Pick the entry named after your server. The second line shows your gamertag and **Bedrock Voice Chat**.

Desktop, mobile and console all connect this way. Consoles need no extra setup. See [console and mobile](/wiki/platforms/console-and-mobile/).

:::caution
Bedrock Voice Chat only works when you join the world through the Worlds tab. Connecting to your Aternos server directly prevents Bedrock Voice Chat from working.
:::

:::note
On Desktop and Mobile devices, if the LAN server does not appear, you can connect to `127.0.0.1:28282` instead from the `Servers` tab.
:::
