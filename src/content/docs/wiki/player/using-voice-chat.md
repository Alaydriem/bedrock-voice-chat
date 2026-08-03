---
title: Using voice chat
description: Proximity ranges, muting, deafening, and per-player volume.
sidebar:
  label: Using voice chat
  order: 3
---

You hear two sets of people: players near you in the world, and anyone in your group.

<div class="shot"><span>BVC dashboard showing the player list with proximity and group members</span></div>

## Proximity

BVC maps each player's mono audio into 3D space using your position and facing against theirs. Someone to your left sounds like they are on your left.

With default settings:

| Distance | What you hear |
|---|---|
| 0–24 blocks | Full volume |
| 24–48 blocks | Fading, down to 40 dB quieter at the edge |
| Beyond 48 blocks | Nothing |

Stereo panning fades in between 8 and 24 blocks, so someone standing on top of you sounds centred rather than hard left or right.

Operators can change all of these. Yours may differ — see the `voice.spatial_audio` block in the [configuration reference](/wiki/reference/configuration/#voicespatial_audio).

## Groups

In a group you hear everyone in it at full volume with no spatial positioning, wherever they are.

Groups are not exclusive. You still hear nearby players and they still hear you. See [Groups](/wiki/player/groups/).

## Muting and deafening yourself

Both controls are at the bottom left of the secondary sidebar.

**Mute** stops your microphone reaching the server. **Deafen** mutes everything incoming.

Both can be bound to a key, along with push-to-talk. See [Settings](/wiki/player/settings/).

## Adjusting other players

Each player tile has its own controls. You can mute someone outright, or set their volume anywhere from 0% to 150%.

Set the volume while they are inside 24 blocks. That is their unattenuated level, and it is what distance falloff is applied to afterwards — adjusting while they are far away means guessing at a number that will change as they walk toward you.

Muting someone only affects what **you** hear. It does not stop them hearing you.
