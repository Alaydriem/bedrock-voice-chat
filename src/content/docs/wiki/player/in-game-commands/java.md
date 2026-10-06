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

## Admin

Server console only. Players and command blocks cannot run these commands, and do not see them.

Requires the [embedded server](/wiki/server/java-mod/#integrated-or-external-bvc-server). With an external server, run [`admin bootstrap`](/wiki/server/players-and-permissions/#bootstrapping-the-first-admin) on the BVC host instead.

| Command | Does |
|---|---|
| `/bvc admin grant <gamertag>` | Give the player the `admin` permission. Creates the player if BVC does not know them. |
| `/bvc admin revoke <gamertag>` | Remove the player's `admin` override. The [server-wide default](/wiki/server/players-and-permissions/#server-wide-defaults) applies again. |
| `/bvc admin deny <gamertag>` | Refuse the player `admin`, even when the default allows it. |

`<gamertag>` is the Xbox gamertag the player signs in to BVC with. It can differ from their name in game. It is case-sensitive. Put it in quotes if it contains a space:

```
/bvc admin grant "Some Name"
```

`grant` creates the player record, so the player can also sign in. `revoke` and `deny` change only existing players. For a gamertag BVC does not know, they reply `<gamertag> is not a BVC player; nothing was changed.`
