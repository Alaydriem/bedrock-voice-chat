---
title: "Using the jukebox"
description: "Play audio clips in-world through the BVC audio player block."
source: https://www.bedrockvoicechat.com/wiki/player/using-the-jukebox/
---
The Jukebox feature of Bedrock Voice Chat lets you play your own audio files in your world. Players near the jukebox hear it louder than players far away. You can use it for scripted games, for example a Decked Out style game that you build with redstone and hoppers.

To use the jukebox, you need three things:

1. A clip in the server's [audio library](/wiki/player/audio-library/).
2. A BVC disc for that clip.
3. A BVC audio player block to put the disc in.

## Get a disc

```
/bvc:disc <audio_id>
```

`audio_id` is the ID of the clip in the audio library. You get a music disc with the name `BVC: <audio_id>`.

<div class="window"><img src="/assets/wiki/mc-disc.jpg" alt="A BVC disc in the hotbar, with its BVC: audio_id name visible" /></div>

## Craft the audio player

### Bedrock

On Bedrock Edition, craft the BVC Audio Player first.

<div class="window"><img src="/assets/wiki/mc-audio-player-recipe.jpg" alt="Crafting recipe for the BVC audio player block" /></div>

### Java

On Java Edition, use the normal Minecraft jukebox.

## Play a clip

1. Place the block.
2. Put the disc in.

<div class="window"><img src="/assets/wiki/mc-jukebox.jpg" alt="A jukebox placed in the world, with a BVC disc in hand" /></div>

All players in range hear the clip through BVC. The clip gets quieter with distance, the same as a voice. See [using voice chat](/wiki/player/using-voice-chat/) for the ranges.

To stop the clip, take the disc out. If the server restarts, the clip starts to play again.

## Volume

Open **Settings → Audio settings** to set one volume for all jukeboxes, or to mute all of them. The volume of voices does not change.

To change the volume in game, use these commands:

| Bedrock | Java |
|---|---|
| `/bvc:jukeboxmute <true\|false>` | `/bvc jukebox mute <true\|false>` |
| `/bvc:jukeboxvolume <0-150>` | `/bvc jukebox volume <0-150>` |

On Bedrock, the **Jukebox music** row of `/bvc:panel` has both controls.

:::caution
All players **MUST** have the BVC app open and connected to hear a jukebox. A player without the app hears nothing.

A disc can play only a clip from the server's library. It cannot play a web link or a file on your device. To add clips, you need the `audio_upload` permission. See [audio library](/wiki/player/audio-library/).
:::

One player can play a maximum of 5 clips at the same time. The server operator can change this limit.

## For operators

The audio player block and the disc command come from the Addon or the Java mod. To change where clips are stored, or the maximum number of clips for each player, see the [`audio` configuration](/wiki/reference/configuration/#audio). To control who can add and delete clips, see [players and permissions](/wiki/server/players-and-permissions/).