---
title: LAN and local worlds
description: Why BVC does not work on LAN, locally hosted, or single-player worlds.
sidebar:
  label: LAN and local worlds
  order: 7
---

Bedrock Voice Chat does not work on LAN worlds, locally hosted worlds or single-player worlds. These are worlds that one player hosts on their own device. Friends can join only while the host is online.

:::note[Two different things called LAN]
A **LAN world** is a world that a game client hosts. BVC does not support it.

Minecraft's **Worlds** tab also lists sessions that it finds on your network. Bedrock Voice Chat Connect sessions show in this list. These sessions work with BVC. See [Realms](/wiki/platforms/realms/) and [Aternos](/wiki/platforms/aternos/).
:::

## Reason

Bedrock Voice Chat needs an Addon on the server to send player positions. Proximity chat cannot work without positions, and only the Addon sends them.

A world that the Minecraft game client hosts cannot load server Addons. On all platforms, there is no place to install one.

### Bedrock Voice Chat Connect

Realms and Aternos use Bedrock Voice Chat Connect. Connect does not work with LAN worlds.

You can upload a world with the Addon to a Realm or an Aternos server. Connect then sends the data from that Addon to BVC. You cannot upload a world to a LAN host. No Addon runs on a LAN world.

## Use Bedrock Dedicated Server

Download Bedrock Dedicated Server (BDS) and start it. It can run on the computer that hosted the LAN world.

You need:

1. A **Bedrock Dedicated Server** or a **Java server**. See [Running a server](/wiki/server/).
2. The **BVC server**, on a host that players can reach.
3. The **BVC app** on the device of each player.

A local Bedrock Dedicated Server does not stop working after a Minecraft update. You choose when to update it.

## Future support

Every known solution needs code that runs on the world host. The Minecraft game client does not allow this.

To fund work on this problem, [support the project](/wiki/start/supporting-bvc/).
