---
title: LAN and local worlds
description: Why BVC does not work on LAN, locally hosted, or single-player worlds.
sidebar:
  label: LAN and local worlds
  order: 7
---

**BVC does not work on LAN worlds, locally hosted worlds, or single-player worlds.** That covers any world where one player hosts from their own device and others join — sometimes described as a world where "the host must be online for friends to join".

## Why

BVC needs a server-side Addon to report where players are. Proximity chat is impossible without position data, and the Addon is the only thing that produces it.

A world hosted from the Minecraft game client cannot load server-side Addons. There is nowhere to install it, on any platform.

### Why proxying does not rescue it

Realms and Aternos work through Proxy Connect, which intercepts Bedrock's network protocol. It is reasonable to ask why that does not work here.

It works there because a Realm and an Aternos instance are real servers you can **upload a world to, Addon included**. The proxy carries the position data the Addon produces. A LAN world has no upload path, so there is no Addon, so there is nothing for a proxy to carry.

The proxy moves data. It does not create it.

## What to do instead

Run Bedrock Dedicated Server. It is a download and a run command, and you can run it on the same machine you were hosting the LAN world from.

You need:

1. A **Bedrock Dedicated Server** or a **Java server** — see [Running a server](/wiki/server/).
2. The **BVC server**, on a reachable host.
3. The **BVC app** on each player's device.

Running BDS locally also removes every caveat that applies to Realms and Aternos: no protocol lag, no version-locked Addon, no breakage on Minecraft update day.

## Will this be supported?

We would like to solve it. There is no approach today that does not require code running on the world host, and the game client does not allow that.

If it matters to you, [supporting the project](/wiki/start/supporting-bvc/) is what funds time spent on problems like this.
