---
title: Bedrock commands
description: "The /bvc: commands and the in-game control panel on Bedrock."
sidebar:
  label: Bedrock
  order: 2
---

Bedrock commands are flat and namespaced with a colon. Type `/bvc:` and the game autocompletes the list.

Requires a server running the [Bedrock Addon](/wiki/server/bedrock-addon/).

## The panel

```
/bvc:panel
```

Opens a form with the same controls as the app: mute, deafen, record, and per-player volume. Easier than typing, and it lists who is actually in range.

<div class="shot"><span>The /bvc:panel control panel open in-game</span></div>

:::tip
On a console this is the practical way to use BVC. Everything else needs typing, and a controller makes that slow.
:::

## Your own audio

| Command | Does |
|---|---|
| `/bvc:mute <on\|off>` | Mute or unmute your microphone. |
| `/bvc:deafen <on\|off>` | Deafen or undeafen — mutes everyone. |
| `/bvc:record <on\|off>` | Start or stop recording. Desktop only. |

## Other players

| Command | Does |
|---|---|
| `/bvc:volume <player> <0-100>` | Set your local volume for one player. |
| `/bvc:hear <player> <on\|off>` | Choose whether you hear them at all. |

Both are local to you.

The range here is 0–100, where the app's per-player slider goes to 150%. The command covers the normal range.

## Groups

| Command | Does |
|---|---|
| `/bvc:groupcreate` | Create a group and print its share code. |
| `/bvc:groupjoin <code>` | Join a group with its code. |
| `/bvc:groupleave` | Leave the group you are in. |

Read the share code out, or paste it in chat. See [Groups](/wiki/player/groups/).

## Audio discs

```
/bvc:disc <audio_id>
```

Gives you a disc bound to a clip from the [audio library](/wiki/player/audio-library/). See [using the jukebox](/wiki/player/using-the-jukebox/).

## Requirements

These come from the Bedrock Addon, so the server has to be running it. You also need the BVC app running and signed in — the commands drive the app rather than replacing it.
