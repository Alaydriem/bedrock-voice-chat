---
title: Using the jukebox
description: Play audio clips in-world through the BVC audio player block.
sidebar:
  label: Using the jukebox
  order: 6
---

BVC can play audio clips in the world. The sound comes out of a block and everyone nearby hears it positioned the same way voices are.

You need three things:

1. A clip in the server's [audio library](/wiki/player/audio-library/).
2. A BVC disc bound to that clip.
3. A BVC audio player block to put the disc in.

## Get a disc

```
/bvc:disc <audio_id>
```

`audio_id` is the clip's ID from the audio library. You get a music disc named `BVC: <audio_id>`. It looks like an ordinary disc but carries the binding.

Any player can run this. It does not require cheats or operator status.

<div class="shot"><span>Inventory showing a BVC disc, with its <code>BVC: &lt;audio_id&gt;</code> name visible</span></div>

Run `/bvc:disc` with no argument and it prints the usage. If the clip ID does not exist in the library the disc is still created, but it will not play.

## Craft the audio player

The BVC audio player is a craftable block added by the Addon. It is the only block a BVC disc plays in — a vanilla jukebox has no way to reach the voice server.

<div class="shot"><span>Crafting recipe for the BVC audio player block</span></div>

## Play it

Place the block, put the disc in, and the clip starts. Everyone in range hears it through BVC with the same distance falloff as voice. See [using voice chat](/wiki/player/using-voice-chat/) for the ranges.

Take the disc out to stop it. It returns to your inventory with the binding intact, so you can carry it elsewhere and play it again.

Playback survives a server restart. A powered player block picks its clip back up rather than sitting silent.

## Limits

Everyone needs the BVC app. The audio travels through BVC, not through Minecraft, so a player without the app connected hears nothing regardless of their in-game volume.

Clips come from the server's library. You cannot point a disc at a URL or a local file. Adding clips needs the `audio_upload` permission — see [audio library](/wiki/player/audio-library/).

Operators set `max_concurrent_per_uuid` (default 5), which caps how many clips one player can have going at once.

## For operators

The audio player block and the disc command come from the Addon or Java mod.

Clip storage and the concurrency cap are configured under the `audio` block — see the
[configuration reference](/wiki/reference/configuration/#audio).

Who can add clips is a permissions question, not a jukebox one: `audio_upload` and
`audio_delete` govern the library that discs draw from. Both are covered in
[audio library](/wiki/player/audio-library/), and granted with
[`bvc permission`](/wiki/server/players-and-permissions/).
