---
title: Bedrock commands
description: "The /bvc: commands and the in-game control panel on Bedrock."
sidebar:
  label: Bedrock
  order: 2
---

Bedrock commands are flat and namespaced with a colon. Type `/bvc:` and the game autocompletes the list.

Requires a server running the [Bedrock Addon](/wiki/server/bedrock-addon/), and the BVC app running and signed in.

## The panel

```
/bvc:panel
```

Opens a form with the same controls as the app: mute, deafen, record, jukebox music, and per-player volume. It lists who is in range.

<div class="shot"><span>The /bvc:panel control panel open in-game</span></div>

:::tip
On a console, use the panel. Everything else needs typing, which is slow on a controller.
:::

## Your own audio

| Command | Does |
|---|---|
| `/bvc:mute <on\|off>` | Mute or unmute your microphone. |
| `/bvc:deafen <on\|off>` | Deafen or undeafen. Mutes everyone. |
| `/bvc:record <on\|off>` | Start or stop recording. Desktop only. |

## Other players

| Command | Does |
|---|---|
| `/bvc:volume <player> <0-150>` | Set your local volume for one player. |
| `/bvc:hear <player> <on\|off>` | Choose whether you hear them at all. |

Both are local to you.

100 leaves a player untouched. Above that boosts a quiet one, to 150 at most. The app's per-player slider covers the same range.

## Jukebox music

| Command | Does |
|---|---|
| `/bvc:jukeboxmute <on\|off>` | Mute or unmute jukebox music. |
| `/bvc:jukeboxvolume <0-150>` | Set how loud jukebox music plays. |

One setting for every jukebox at once, and only for you. Voices are unaffected.

The panel has the same two controls on its **Jukebox music** row. See [using the jukebox](/wiki/player/using-the-jukebox/).

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
