---
title: "Realms"
description: "Run BVC on a Minecraft Realm using the no-net Addon and Bedrock Voice Chat Connect."
source: https://www.bedrockvoicechat.com/wiki/platforms/realms/
---
Bedrock Voice Chat supports Minecraft Realms with the no-net Addon and Bedrock Voice Chat Connect. You can use proximity voice chat on a Realm that you do not control. Supported since 1.0.0-beta.15.

:::caution
Realms update automatically. After a Minecraft update, voice chat on your Realm **stops** until a matching Bedrock Voice Chat version is available. Check [version support](/wiki/platforms/version-support/) before you move your server to a Realm.
:::

## Before you start

1. Install and configure the [BVC server](/wiki/server/installation/). Realms also need a BVC server.
2. Install the [BVC app](/wiki/start/downloads/).
3. Build the no-net world with the [no-net Addon](/wiki/server/nonet-addon/) guide.
4. Add each Realm player on the BVC server. See [players and permissions](/wiki/server/players-and-permissions/).

## Upload to the Realm

1. In Minecraft, open the settings of your Realm.
2. Replace the Realm world with the world that you built.
3. When the Realm has saved the world, leave the Realm.

## Connect

:::note
If you disconnect from Bedrock Voice Chat Connect, wait about five minutes before you connect again. Realms use NetherNet, and NetherNet does not accept a new connection during this time.
:::

1. Sign in to the BVC app with the BVC server from [Before you start](#before-you-start).
2. Select the Settings cog at the bottom left. Then select **Voice Chat Connect** in the sidebar. The first time, the app asks you to sign in again.
3. Select your Realm.

<div class="window"><img src="/assets/wiki/settings-connect.jpg" alt="Voice Chat Connect page with a Realm selected" /></div>

## Open Minecraft

The BVC app shows the session on your local network. Minecraft lists it under **Worlds**.

1. Open Minecraft on the same network as the device that runs BVC.
2. Go to **Worlds**.
3. Select the entry with the name of your Realm. The second line shows your gamertag and **Bedrock Voice Chat**.

Desktop, mobile and console all connect this way. Consoles need no extra setup. See [console and mobile](/wiki/platforms/console-and-mobile/).

:::danger
Join the world from the **Worlds** tab. If you connect to your Realm directly, Minecraft works but voice chat does not.
:::

:::note
On desktop and mobile, if the session is not in **Worlds**, open the **Servers** tab and add `127.0.0.1` with port `28282`.
:::