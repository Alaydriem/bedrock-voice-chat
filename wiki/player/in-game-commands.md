---
title: "In-game commands"
description: "Control BVC from inside Minecraft without alt-tabbing."
source: https://www.bedrockvoicechat.com/wiki/player/in-game-commands/
---
The Bedrock Addon and the Java mod let you control BVC with commands in the chat bar. You can also use these commands in scripted events.

The commands are different on each edition:

- **[Bedrock](/wiki/player/in-game-commands/bedrock/)**: commands start with `/bvc:`. Example: `/bvc:mute true`
- **[Java](/wiki/player/in-game-commands/java/)**: commands start with `/bvc` and a space. Example: `/bvc mute true`

## Commands on both editions

All players can use every command. No command needs cheats or operator status.

The BVC app must be open and signed in. The commands control the app.

| Action | Bedrock | Java |
|---|---|---|
| Mute yourself | `/bvc:mute <true\|false>` | `/bvc mute <true\|false>` |
| Deafen yourself | `/bvc:deafen <true\|false>` | `/bvc deafen <true\|false>` |
| Start or stop recording | `/bvc:record <true\|false>` | `/bvc record <true\|false>` |
| Set a player's volume | `/bvc:volume <player> <0-150>` | `/bvc volume <player> <0-150>` |
| Hear a player or not | `/bvc:hear <player> <true\|false>` | `/bvc hear <player> <true\|false>` |
| Mute jukebox music | `/bvc:jukeboxmute <true\|false>` | `/bvc jukebox mute <true\|false>` |
| Set jukebox volume | `/bvc:jukeboxvolume <0-150>` | `/bvc jukebox volume <0-150>` |
| Create a group | `/bvc:groupcreate` | `/bvc group create` |
| Join a group | `/bvc:groupjoin <code>` | `/bvc group join <code>` |
| Leave a group | `/bvc:groupleave` | `/bvc group leave` |
| Get an audio disc | `/bvc:disc <audio_id>` | `/bvc disc <audio_id>` |
| Open a control panel | `/bvc:panel` | None |

:::note
The control panel is only on Bedrock. On Java, use the app window.
:::

Volume, hear, and the jukebox commands change only what you hear. When you make a player quieter, other players hear no change, and that player can still hear you.

To join a [Private Group](/wiki/player/groups/#private-groups), a member must admit you in the BVC app. The join command starts the request, and the app shows the code.

Recording is available only on desktop. You can type the command on all devices. It does nothing on a device that cannot record, or on a server that has [turned recording off](/wiki/creator/recording/).