---
title: LAN and local worlds
description: Why BVC does not work on LAN, locally hosted, or single-player worlds.
sidebar:
  label: LAN and local worlds
  order: 7
---

**BVC does not work on LAN worlds, locally hosted worlds, or single-player worlds.** That covers any world where one player hosts from their own device and others join — sometimes described as a world where "the host must be online for friends to join".

:::note[Two different things called LAN]
A **LAN world** is a world hosted by a game client. BVC does not support it.

Minecraft's **Worlds** tab also lists sessions found on your network. Bedrock Voice Chat Connect sessions appear there. Those work. See [Realms](/wiki/platforms/realms/) and [Aternos](/wiki/platforms/aternos/).
:::

## Why

Bedrock Voice Chat needs a server-side Addon to report where players are. Proximity chat is impossible without position data, and the Addon is the only thing that produces it.

A world hosted from the Minecraft game client cannot load server-side Addons. There is nowhere to install one, on any platform.

### Bedrock Voice Chat Connect

Realms and Aternos work through Bedrock Voice Chat Connect, which intercepts Bedrock's network protocol. It does not help here.

A Realm and an Aternos instance are real servers you can **upload a world to, Addon included**. Bedrock Voice Chat Connect carries the position data that Addon produces. A LAN world has no upload path, leaving no Addon and nothing to carry.

## The alternative

Run Bedrock Dedicated Server. It is a download and a run command, and you can run it on the same machine you were hosting the LAN world from.

You need:

1. A **Bedrock Dedicated Server** or a **Java server** — see [Running a server](/wiki/server/).
2. The **BVC server**, on a reachable host.
3. The **BVC app** on each player's device.

Running BDS locally also removes every caveat that applies to Realms and Aternos: no protocol lag, no version-locked Addon, no breakage on Minecraft update day.

## Will this be supported?

We would like to solve it. There is no approach today that does not require code running on the world host, and the game client does not allow that.

If it matters to you, [supporting the project](/wiki/start/supporting-bvc/) is what funds time spent on problems like this.
