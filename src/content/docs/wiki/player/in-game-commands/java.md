---
title: Java commands
description: The /bvc commands on Fabric and PaperMC servers.
sidebar:
  label: Java
  order: 3
---

Java commands nest under a single `/bvc` root. Type `/bvc ` and tab-complete the rest.

Requires a server running the [Java mod](/wiki/server/java-mod/), on either Fabric or PaperMC. The commands are identical on both.

## Your own audio

| Command | Does |
|---|---|
| `/bvc mute <on\|off>` | Mute or unmute your microphone. |
| `/bvc deafen <on\|off>` | Deafen or undeafen — mutes everyone. |
| `/bvc record <on\|off>` | Start or stop recording. Desktop only. |

## Other players

| Command | Does |
|---|---|
| `/bvc volume <player> <0-100>` | Set your local volume for one player. |
| `/bvc hear <player> <on\|off>` | Choose whether you hear them at all. |

Both are local to you. Turning someone down does not change what anyone else hears, and it does not stop them hearing you.

The range is 0–100, where the app's per-player slider goes to 150%. The command covers the normal range.

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

Must be run by a player, since it puts an item in your inventory. From the console, use the give form instead:

```
/bvc give <player> <audio_id>
```

## No control panel

Bedrock has `/bvc:panel`, which opens an in-game form. Java has no equivalent — that panel is built on Bedrock's server-side form API, and Java has no counterpart to it.

Use the app window on Java. Everything the panel does is there.

## Requirements

The BVC app must be running and signed in. These commands drive the app rather than replacing it.
