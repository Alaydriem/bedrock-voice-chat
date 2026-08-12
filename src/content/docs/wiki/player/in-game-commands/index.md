---
title: In-game commands
description: Control BVC from inside Minecraft without alt-tabbing.
sidebar:
  label: Overview
  order: 1
---

Both editions let you drive BVC from the chat bar. Useful on a handheld, and the only practical option on a console.

The syntax differs between editions:

- **[Bedrock](/wiki/player/in-game-commands/bedrock/)** — flat, namespaced with a colon: `/bvc:mute true`
- **[Java](/wiki/player/in-game-commands/java/)** — nested under one root: `/bvc mute true`

## Common to both

Every command works for any player. None require cheats or operator status.

The BVC app must be running and signed in. These commands drive the app.

| Capability | Bedrock | Java |
|---|---|---|
| Mute yourself | `/bvc:mute <on>` | `/bvc mute <on>` |
| Deafen yourself | `/bvc:deafen <on>` | `/bvc deafen <on>` |
| Toggle recording | `/bvc:record <on>` | `/bvc record <on>` |
| Set a player's volume | `/bvc:volume <player> <0-150>` | `/bvc volume <player> <0-150>` |
| Hear a player or not | `/bvc:hear <player> <on>` | `/bvc hear <player> <on>` |
| Mute jukebox music | `/bvc:jukeboxmute <on>` | `/bvc jukebox mute <on>` |
| Set jukebox volume | `/bvc:jukeboxvolume <0-150>` | `/bvc jukebox volume <0-150>` |
| Create a group | `/bvc:groupcreate` | `/bvc group create` |
| Join a group | `/bvc:groupjoin <code>` | `/bvc group join <code>` |
| Leave a group | `/bvc:groupleave` | `/bvc group leave` |
| Get an audio disc | `/bvc:disc <audio_id>` | `/bvc disc <audio_id>` |
| Open a control panel | `/bvc:panel` | — |

:::note
The control panel is Bedrock only. It uses Bedrock's server-side form API, which Java has no counterpart to. Java players use the app window.
:::

Volume, hear, and the jukebox controls are local to you. Turning someone down does not change what anyone else hears, and does not stop them hearing you.

Recording is desktop only. The command exists everywhere and does nothing on a device that cannot record, or on a server that has [turned recording off](/wiki/creator/recording/).
