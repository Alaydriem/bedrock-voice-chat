---
title: Aternos and no-net hosts
description: Run BVC on Aternos and other hosts that cannot enable @minecraft/server-net.
sidebar:
  label: Aternos
  order: 4
---

Bedrock Voice Chat supports Aternos and other free hosts that do not allow `@minecraft/server-net`. Use the no-net Addon and Bedrock Voice Chat Connect. Supported since 1.0.0-beta.15.

The BVC app runs a local Bedrock server. Minecraft connects to the app, and the app connects to Aternos.

:::caution
Support for each Minecraft release comes after the release. If your host updates Minecraft automatically, voice chat **stops** on the day of the update. Check [version support](/wiki/platforms/version-support/) before you move your server to Aternos.
:::

## Before you start

1. Install and configure the [BVC server](/wiki/server/installation/).
2. Install the [BVC app](/wiki/start/downloads/).
3. Build the no-net world with the [no-net Addon](/wiki/server/nonet-addon/) guide.

## Upload the world

1. Find the `.mcworld` file that you exported, and change the extension to `.zip`.
2. Upload it to Aternos.

   <div class="window"><img src="/assets/wiki/aternos-worlds.jpg" alt="Aternos world upload page" /></div>

3. Set the Aternos server version to the Minecraft version that the Addon supports. Each Addon works with one Minecraft version only. If the versions do not match, voice chat does not work.

Check [version support](/wiki/platforms/version-support/) for the version to select.

## Connect

1. Sign in to the BVC app with the BVC server from [Before you start](#before-you-start).
2. Select the Settings cog at the bottom left. Then select **Voice Chat Connect** in the sidebar. The first time, the app asks you to sign in again.
3. Select the add button at the top. Enter your Aternos domain and port.

   <div class="window"><img src="/assets/wiki/settings-connect-aternos-edit.jpg" alt="Bedrock Voice Chat Connect add-server dialog with an Aternos address" /></div>

4. Select **Connect**. The app shows the connection details.

   <div class="window"><img src="/assets/wiki/settings-connect-aternos-connected.jpg" alt="Bedrock Voice Chat Connect showing connection information for a connected server" /></div>

## Open Minecraft

The BVC app shows the session on your local network. Minecraft lists it under **Worlds**.

1. Open Minecraft on the same network as the device that runs BVC.
2. Go to **Worlds**.
3. Select the entry with the name of your server. The second line shows your gamertag and **Bedrock Voice Chat**.

Desktop, mobile and console all connect this way. Consoles need no extra setup. See [console and mobile](/wiki/platforms/console-and-mobile/).

:::danger
Join the world from the **Worlds** tab. If you connect to your Aternos server directly, Minecraft works but voice chat does not.
:::

:::note
On desktop and mobile, if the session is not in **Worlds**, open the **Servers** tab and add `127.0.0.1` with port `28282`.
:::
