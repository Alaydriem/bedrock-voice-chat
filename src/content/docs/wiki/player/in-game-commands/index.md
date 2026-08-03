---
title: In-game commands
description: Control BVC from inside Minecraft without alt-tabbing.
sidebar:
  label: Overview
  order: 1
---

Both editions let you drive BVC from the chat bar. Useful on a handheld, and the only practical option on a console.

The commands do the same things on both, but the syntax differs because the two command systems do:

- **[Bedrock](/wiki/player/in-game-commands/bedrock/)** — flat and namespaced with a colon: `/bvc:mute true`
- **[Java](/wiki/player/in-game-commands/java/)** — nested under one root: `/bvc mute true`

## Common to both

Every command works for any player. None require cheats or operator status.

You still need the BVC app running and signed in. These drive the app; they do not replace it.

| Capability | Bedrock | Java |
|---|---|---|
| Mute yourself | `/bvc:mute <on>` | `/bvc mute <on>` |
| Deafen yourself | `/bvc:deafen <on>` | `/bvc deafen <on>` |
| Toggle recording | `/bvc:record <on>` | `/bvc record <on>` |
| Set a player's volume | `/bvc:volume <player> <0-100>` | `/bvc volume <player> <0-100>` |
| Hear a player or not | `/bvc:hear <player> <on>` | `/bvc hear <player> <on>` |
| Create a group | `/bvc:groupcreate` | `/bvc group create` |
| Join a group | `/bvc:groupjoin <code>` | `/bvc group join <code>` |
| Leave a group | `/bvc:groupleave` | `/bvc group leave` |
| Get an audio disc | `/bvc:disc <audio_id>` | `/bvc disc <audio_id>` |
| Open a control panel | `/bvc:panel` | — |

:::note
The control panel is Bedrock only. It is built on Bedrock's server-side form API, which has no Java equivalent — Java players use the app window instead.
:::

Volume and hear are local to you. Turning someone down does not change what anyone else hears, and it does not stop them hearing you.

Recording is desktop only. The command exists on every platform and does nothing on a device that cannot record.
