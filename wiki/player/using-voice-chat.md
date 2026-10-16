---
title: "Using voice chat"
description: "Proximity ranges, muting, deafening, and per-player volume."
source: https://www.bedrockvoicechat.com/wiki/player/using-voice-chat/
---
Bedrock Voice Chat lets you hear people near you in the world, and people in groups that you create or join.

<div class="window"><img src="/assets/wiki/dashboard-group.jpg" alt="BVC dashboard showing the player list with proximity and group members" /></div>

## Proximity

BVC uses your position, the direction you face, and the position of each player to put their voice around you. A player on your left sounds like they are on your left.

With the default settings:

| Distance | What you hear |
|---|---|
| 0 to 24 blocks | Full volume. |
| 24 to 48 blocks | The voice gets quieter. At 48 blocks, it is very quiet. |
| More than 48 blocks | Nothing. |

Left-right positioning starts at 8 blocks and increases up to 24 blocks. A player very close to you sounds like they are in front of you.

Server operators can change all of these values. See the `voice.spatial_audio` block in the [configuration reference](/wiki/reference/configuration/#voicespatial_audio).

## Groups

In a group, you hear all group members at full volume, wherever they are. Their voices do not move left or right. You also still hear nearby players, and they still hear you. See [Groups](/wiki/player/groups/).

## Mute and deafen yourself

Both buttons are at the bottom of the sidebar on the left.

**Mute** stops BVC from sending your voice. **Deafen** stops all sound from other players.

You can set a key for each button, and for push-to-talk. See [Settings](/wiki/player/settings/).

<div class="window"><img src="/assets/wiki/mute-deafen.jpg" alt="The Mute and Deafen buttons within the app." /></div>

## Adjust another player

You can change the volume of any player who comes near you. You can mute them, or make them louder or quieter.

<div class="window"><img src="/assets/wiki/dashboard-earshot.jpg" alt="Adjusting player volume" /></div>

:::tip
Be sure to change a player's volume while they are within 24 blocks of you. At this distance, you hear them at full volume.
:::

:::caution
When you mute a player, only **you** stop hearing them. They can still hear you.
:::

Your changes stay after the player walks away or leaves the game. To find a player who is not on the screen, or to undo a mute, open **Settings → Player audio levels**. It lists every player you have heard, and has a button to reset all of them.