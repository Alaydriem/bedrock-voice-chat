---
title: Java commands
description: The /bvc commands on Fabric and PaperMC servers.
sidebar:
  label: Java
  order: 3
---

On Java, all BVC commands start with `/bvc` and a space. Type `/bvc ` and press Tab to complete the rest.

These commands need the [Java mod](/wiki/server/java-mod/) on a Fabric or PaperMC server. The BVC app must be open and signed in. The commands are the same on Fabric and PaperMC.

## Your own audio

| Command | Result |
|---|---|
| `/bvc mute <true\|false>` | Mutes or unmutes your microphone. |
| `/bvc deafen <true\|false>` | Deafens or undeafens you. When you are deafened, you hear no one. |
| `/bvc record <true\|false>` | Starts or stops recording. Desktop only. |

## Other players

| Command | Result |
|---|---|
| `/bvc volume <player> <0-150>` | Sets the volume of one player for you. |
| `/bvc hear <player> <true\|false>` | Sets whether you hear that player. |

These settings change only what you hear. When you make a player quieter, other players hear no change, and that player can still hear you.

At 100, a player plays at normal volume. To make a quiet player louder, set a value above 100. The maximum is 150. The volume slider for each player in the app has the same range.

## Jukebox music

| Command | Result |
|---|---|
| `/bvc jukebox mute <true\|false>` | Mutes or unmutes jukebox music. |
| `/bvc jukebox volume <0-150>` | Sets the volume of jukebox music. |

This setting applies to all jukeboxes at the same time. It changes only what you hear. It does not change the volume of voices.

These are the same two controls as **Settings → Audio settings** in the app. See [using the jukebox](/wiki/player/using-the-jukebox/).

## Groups

| Command | Result |
|---|---|
| `/bvc group create` | Creates a group and shows its share code. |
| `/bvc group join <code>` | Joins a group with its code. For a Private Group, asks to join in the BVC app. |
| `/bvc group leave` | Leaves your current group. |

For a Private Group, a member must admit you in the BVC app, after you both compare a code there. The game replies `Your code for <name> is in the BVC app.` If the app is not open and signed in, it replies `Open the BVC app to join <name>.` See [Ask to join from the game](/wiki/player/groups/#ask-to-join-from-the-game).

## Audio discs

```
/bvc disc <audio_id>
```

This command gives you a disc that plays a clip from the [audio library](/wiki/player/audio-library/). See [using the jukebox](/wiki/player/using-the-jukebox/).

Only a player can run `/bvc disc`. To give a disc from the server console, use this command:

```
/bvc give <player> <audio_id>
```

## Admin

These commands are for server operators. You can run them only from the server console. Players and command blocks cannot run or see them.

They need the [embedded server](/wiki/server/java-mod/#external-or-embedded-server). With an external server, run [`admin bootstrap`](/wiki/server/players-and-permissions/#bootstrapping-the-first-admin) on the BVC host.

| Command | Result |
|---|---|
| `/bvc admin grant <gamertag>` | Gives the player the `admin` permission. Creates the player if BVC does not know them. |
| `/bvc admin revoke <gamertag>` | Removes the `admin` setting for this player. The [server-wide default](/wiki/server/players-and-permissions/#server-wide-defaults) then applies. |
| `/bvc admin deny <gamertag>` | Refuses `admin` to the player, even when the default allows it. |

`<gamertag>` is the Xbox gamertag that the player uses to sign in to BVC. It can be different from their name in game. It is case-sensitive. If it contains a space, put it in quotes:

```
/bvc admin grant "Some Name"
```

`grant` creates the player record. After this, the player can also sign in to BVC. `revoke` and `deny` change only existing players. If BVC does not know the gamertag, the command shows `<gamertag> is not a BVC player; nothing was changed.`
