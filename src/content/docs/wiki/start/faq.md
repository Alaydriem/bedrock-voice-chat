---
title: FAQ
description: Short answers to the questions that come up most.
sidebar:
  label: FAQ
  order: 2
---

## Does it work on Realms?

Yes, since 1.0.0-beta.15, using the no-net Addon and Realms Connect. Support lags each Minecraft update. See [Realms](/wiki/platforms/realms/).

## Does it work on Aternos?

Yes, same approach. See [Aternos](/wiki/platforms/aternos/).

## Does it work on a LAN or single-player world?

No. There is nowhere to install the Addon, and BVC never receives position data. [Why](/wiki/platforms/lan-and-local-worlds/).

## Can I use it on Xbox, PlayStation, or Switch?

Yes, with the mobile app running next to the console. There is no on-console client. [Console and mobile](/wiki/platforms/console-and-mobile/).

## Does it work with Geyser?

Yes. Install the Java mod. Floodgate is auto-detected. [Geyser](/wiki/platforms/geyser/).

## Does it work with Simple Voice Chat?

No. They are separate systems. Players on one cannot hear players on the other.

BVC covers both sides itself. Install the [Java mod](/wiki/server/java-mod/) and your Java and Bedrock players hear each other through the BVC apps. The tradeoff: everyone needs the app, where Simple Voice Chat is built into the Java client. That app is also what makes cross-platform and console support possible.

## Is there a public server I can join?

No. There is no official server and no directory of them. Any operator can install BVC, and there is no central place to point you at.

If you already play somewhere you like, ask the owner to install it. The [guide](/wiki/server/) is here.

## What is my login URL?

The address of the **BVC server**, not your Minecraft server, Aternos address, or Realm. Your operator sets it up and gives it to you. [Signing in](/wiki/player/signing-in/).

## Why can I sign in but not hear anyone?

Most often UDP is blocked, or you connected directly to a Realm or Aternos server instead of going through Realms Connect or Proxy Connect. [Troubleshooting](/wiki/player/troubleshooting/).

## Do I need the BVC server if I use the Java mod?

Not always. The Java mod can run BVC embedded in your game server. Good for small always-on servers, poor on low-end hosts. [Java mod](/wiki/server/java-mod/).

You do need one for Realms and Aternos, even though those use the no-net Addon.

## Can I record?

Yes, on desktop, with each player on their own track. Not on mobile. Your server operator can turn recording off. [Recording](/wiki/creator/recording/).

## Can I read in-game chat in the app?

Yes. The app mirrors server chat and sends what you type back into the game. [Chat](/wiki/player/chat/).

## Is it free?

Yes. [Supporting the project](/wiki/start/supporting-bvc/) funds development and gets you early access to builds, which matters most around Minecraft updates.

## Why is there no support the day Minecraft updates?

This affects Realms and Aternos only. Mojang does not document the Bedrock packet protocol and changes it every release. The work cannot start until the update ships. [Version support](/wiki/platforms/version-support/).

Running your own Bedrock Dedicated Server avoids it entirely.
