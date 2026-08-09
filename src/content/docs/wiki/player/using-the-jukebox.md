---
title: Using the jukebox
description: Play audio clips in-world through the BVC audio player block.
sidebar:
  label: Using the jukebox
  order: 6
---

BVC plays audio clips in the world. The sound comes out of a block, positioned the same way voices are.

You need three things:

1. A clip in the server's [audio library](/wiki/player/audio-library/).
2. A BVC disc bound to that clip.
3. A BVC audio player block to put the disc in.

## Get a disc

```
/bvc:disc <audio_id>
```

`audio_id` is the clip's ID from the audio library. You get a music disc named `BVC: <audio_id>`.

Any player can run this. It does not require cheats or operator status.

<div class="shot"><span>Inventory showing a BVC disc, with its <code>BVC: &lt;audio_id&gt;</code> name visible</span></div>

Run `/bvc:disc` with no argument to print the usage. A disc bound to an ID that is not in the library is still created, and will not play.

## Craft the audio player

The BVC audio player is a craftable block added by the Addon. A BVC disc plays in this block only. A vanilla jukebox cannot reach the voice server.

<div class="shot"><span>Crafting recipe for the BVC audio player block</span></div>

## Play a clip

1. Place the block.
2. Put the disc in.

Everyone in range hears it through BVC with the same distance falloff as voice. See [using voice chat](/wiki/player/using-voice-chat/) for the ranges.

Take the disc out to stop it. The binding stays on the disc.

Playback survives a server restart. A powered player block picks its clip back up.

## Volume

Open **Settings → Audio** to set one volume for every jukebox at once, or mute them all. Voices are unaffected.

## Limits

Everyone needs the BVC app. The audio travels through BVC, not through Minecraft. A player without the app connected hears nothing.

Clips come from the server's library. A disc cannot point at a URL or a local file. Adding clips needs the `audio_upload` permission. See [audio library](/wiki/player/audio-library/).

`max_concurrent_per_uuid` caps how many clips one player can have going at once. Default is 5.

## For operators

The audio player block and the disc command come from the Addon or Java mod.

Clip storage and the concurrency cap are the `audio` block. See the [configuration reference](/wiki/reference/configuration/#audio).

`audio_upload` and `audio_delete` govern the library that discs draw from. Both are covered in [audio library](/wiki/player/audio-library/) and granted with [`bvc permission`](/wiki/server/players-and-permissions/).
