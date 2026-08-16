---
title: Console and mobile
description: Using BVC on Xbox, PlayStation, Switch, and mobile devices.
sidebar:
  label: Console and mobile
  order: 6
---

Bedrock Voice Chat supports Xbox, PlayStation, and Switch players through the mobile app. Your phone or tablet is your microphone and speaker, running alongside the console.

Your position comes from the game server. The console does nothing special, and the app knows where you are without being installed on it.

:::note
There is no on-console Bedrock Voice Chat client. Consoles do not allow installable applications of this kind.

If you would like this feature, consider supporting Bedrock Voice Chat on Patreon, or by becoming a YouTube Member.
:::

## Get the app

- **Android** — [Google Play](https://play.google.com/store/apps/details?id=com.alaydriem.bvc.client), open beta
- **iOS / iPadOS** — [TestFlight](https://testflight.apple.com/join/JSG7bVqC)

Sign in with the same Xbox account you play on.

## On a Bedrock Dedicated Server

Nothing console-specific. Join the server as usual, sign into the app, done.

## On Realms or Aternos

These go through Bedrock Voice Chat Connect. The app hosts the session on your network, and it appears in Minecraft under **Worlds**.

### Operator

Nothing to configure. The relay defaults handle it.

### Player

1. Put the console and the phone running BVC on the same network.
2. Open BVC. Open Settings, then **Voice Chat Connect**.
3. Pick your Realm or server. Wait for the connection details.
4. Open Minecraft on the console. Go to **Worlds**.
5. Pick the entry named after your world. The second line shows your gamertag and **Bedrock Voice Chat**.

<div class="shot"><span>Minecraft's Worlds tab showing a BVC session, world name on the first line and gamertag on the second</span></div>

If two people on the network run BVC in the same world, both entries appear. The gamertag on the second line tells them apart.

:::caution
Do **not** enter your Realm or Aternos address directly. That bypasses Bedrock Voice Chat entirely. Minecraft works and nobody hears anyone.
:::

## Playing on mobile

A phone or tablet playing Minecraft can enter a custom server address. Follow the desktop instructions on the [Realms](/wiki/platforms/realms/) and [Aternos](/wiki/platforms/aternos/) pages.

## Recording

:::note
Recording is not available on mobile. Phones and tablets lack the disk throughput for multi-track capture. Record on a desktop instead. See [Recording](/wiki/creator/recording/).
:::
