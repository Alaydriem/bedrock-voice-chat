---
title: Java commands
description: The /bvc commands on Fabric and PaperMC servers.
sidebar:
  label: Java
  order: 3
---

Java commands nest under a single `/bvc` root. Type `/bvc ` and tab-complete the rest.

Requires a server running the [Java mod](/wiki/server/java-mod/) on Fabric or PaperMC, and the BVC app running and signed in. The commands are identical on both loaders.

## Your own audio

| Command | Does |
|---|---|
| `/bvc mute <on\|off>` | Mute or unmute your microphone. |
| `/bvc deafen <on\|off>` | Deafen or undeafen. Mutes everyone. |
| `/bvc record <on\|off>` | Start or stop recording. Desktop only. |

## Other players

| Command | Does |
|---|---|
| `/bvc volume <player> <0-150>` | Set your local volume for one player. |
| `/bvc hear <player> <on\|off>` | Choose whether you hear them at all. |

Both are local to you. Turning someone down does not change what anyone else hears, and does not stop them hearing you.

100 leaves a player untouched. Above that boosts a quiet one, to 150 at most. The app's per-player slider covers the same range.

## Jukebox music

| Command | Does |
|---|---|
| `/bvc jukebox mute <on\|off>` | Mute or unmute jukebox music. |
| `/bvc jukebox volume <0-150>` | Set how loud jukebox music plays. |

One setting for every jukebox at once, and only for you. Voices are unaffected.

These are the same two controls as **Settings → Audio** in the app. See [using the jukebox](/wiki/player/using-the-jukebox/).

## Groups

| Command | Does |
|---|---|
| `/bvc group create` | Create a group and print its share code. |
| `/bvc group join <code>` | Join a group with its code. |
| `/bvc group leave` | Leave the group you are in. |

See [Groups](/wiki/player/groups/).

## Audio discs

```
/bvc disc <audio_id>
```

Gives you a disc bound to a clip from the [audio library](/wiki/player/audio-library/). See [using the jukebox](/wiki/player/using-the-jukebox/).

A player has to run it. From the console, use the give form:

```
/bvc give <player> <audio_id>
```
