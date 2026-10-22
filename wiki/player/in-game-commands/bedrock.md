---
title: "Bedrock commands"
description: "The /bvc: commands and the in-game control panel on Bedrock."
source: https://www.bedrockvoicechat.com/wiki/player/in-game-commands/bedrock/
---
On Bedrock, all BVC commands start with `/bvc:`. Type `/bvc:` and the game shows the list of commands.

These commands need the [Bedrock Addon](/wiki/server/bedrock-addon/) on the server. The BVC app must be open and signed in.

## The panel

```
/bvc:panel
```

This command opens a form with the same controls as the app: mute, deafen, record, jukebox music, and the volume of each player.

<div class="window"><img src="/assets/wiki/mc-bvc-panel.jpg" alt="The /bvc:panel control panel open in-game" /></div>

## Your own audio

| Command | Result |
|---|---|
| `/bvc:mute <true\|false>` | Mutes or unmutes your microphone. |
| `/bvc:deafen <true\|false>` | Deafens or undeafens you. When you are deafened, you hear no one. |
| `/bvc:record <true\|false>` | Starts or stops recording. Desktop only. |

## Other players

| Command | Result |
|---|---|
| `/bvc:volume <player> <0-150>` | Sets the volume of one player for you. |
| `/bvc:hear <player> <true\|false>` | Sets whether you hear that player. |

These settings change only what you hear. They do not change what other players hear.

At 100, a player plays at normal volume. To make a quiet player louder, set a value above 100. The maximum is 150. The volume slider for each player in the app has the same range.

<div class="window"><img src="/assets/wiki/mc-player-volumes.jpg" alt="The Player volumes form, with the Jukebox music row above the player list" /></div>

## Jukebox music

| Command | Result |
|---|---|
| `/bvc:jukeboxmute <true\|false>` | Mutes or unmutes jukebox music. |
| `/bvc:jukeboxvolume <0-150>` | Sets the volume of jukebox music. |

This setting applies to all jukeboxes at the same time. It changes only what you hear. It does not change the volume of voices.

The panel has the same two controls on its **Jukebox music** row. See [using the jukebox](/wiki/player/using-the-jukebox/).

## Groups

| Command | Result |
|---|---|
| `/bvc:groupcreate` | Creates a group and shows its share code. |
| `/bvc:groupjoin <code>` | Joins a group with its code. For a Private Group, asks to join in the BVC app. |
| `/bvc:groupleave` | Leaves your current group. |

Tell other players the share code, or paste it in chat. See [Groups](/wiki/player/groups/).

In the **Groups** form of the panel, a Private Group shows as `[Ask] <name>`. Select it to ask to join. A member must admit you in the BVC app, after you both compare a code there. The game replies `Your code for <name> is in the BVC app.` If the app is not open and signed in, it replies `Open the BVC app to join <name>.` See [Ask to join from the game](/wiki/player/groups/#ask-to-join-from-the-game).

<div class="window"><img src="/assets/wiki/mc-groups.jpg" alt="The Groups form, listing groups with the current group's share code" /></div>

## Audio discs

```
/bvc:disc <audio_id>
```

This command gives you a disc that plays a clip from the [audio library](/wiki/player/audio-library/). See [using the jukebox](/wiki/player/using-the-jukebox/).