---
title: Realms
description: Run BVC on a Minecraft Realm using the no-net Addon and Bedrock Voice Chat Connect.
sidebar:
  label: Realms
  order: 3
---

Bedrock Voice Chat supports Minecraft Realms through the no-net Addon and Bedrock Voice Chat Connect, letting you run proximity voice chat on a Realm you do not control. Supported as of 1.0.0-beta.15 through the no-net addon.

:::caution
Realms auto-update. A Minecraft update **will break** voice on your Realm until a matching Bedrock Voice Chat build ships. Check [version support](/wiki/platforms/version-support/) before committing your community to a Realm.
:::

## Before you start

1. Install and configure the [BVC server](/wiki/server/installation/). Not optional — the no-net path still needs one.
2. Install the [client app](/wiki/start/downloads/).
3. Build the no-net world following the [no-net Addon](/wiki/server/nonet-addon/) guide.
4. Ensure your realms players are manually whitelisted on BVC Server.

## Upload to the Realm

Create your world on the Realm from the world you built, then exit the Realm once it has saved.

## Connect

:::note
Realms use Nethernet, which has a connect cooldown. If you disconnect from Bedrock Voice Chat Connect, expect to wait around five minutes before reconnecting while the Realm is idle.
:::

1. Sign into the BVC client against the server from step 1.
2. Settings cog, bottom left. Then **Voice Chat Connect** in the sidebar. First visit asks you to sign in again.
3. Select your Realm.

<div class="shot"><span>Voice Chat Connect page with a Realm selected</span></div>

## Open Minecraft

The app hosts the session on your network. It appears in Minecraft under **Worlds**.

1. Open Minecraft on the same network as the device running BVC.
2. Go to **Worlds**.
3. Pick the entry named after your Realm. The second line shows your gamertag and **Bedrock Voice Chat**.

Desktop, mobile and console all connect this way. Consoles need no extra setup. See [console and mobile](/wiki/platforms/console-and-mobile/).

:::caution
Bedrock Voice Chat only works when you join the world through the Worlds tab. Connecting to your Realm directly prevents Bedrock Voice Chat from working.
:::

:::note
On Desktop and Mobile devices, if the LAN server does not appear, you can connect to `127.0.0.1:28282` instead from the `Servers` tab.
:::