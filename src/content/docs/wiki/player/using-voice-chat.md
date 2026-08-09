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

BVC places each player's voice in 3D space using your position and facing against theirs. Someone to your left sounds like they are on your left.

With default settings:

| Distance | What you hear |
|---|---|
| 0–24 blocks | Full volume |
| 24–48 blocks | Fading, down to 40 dB quieter at the edge |
| Beyond 48 blocks | Nothing |

Stereo panning fades in between 8 and 24 blocks. Someone standing on top of you sounds centred.

Operators can change all of these. See the `voice.spatial_audio` block in the [configuration reference](/wiki/reference/configuration/#voicespatial_audio).

## Groups

In a group you hear everyone at full volume with no spatial positioning, wherever they are.

Groups are not exclusive. You still hear nearby players and they still hear you. See [Groups](/wiki/player/groups/).

## Mute and deafen yourself

Both controls are at the bottom left of the secondary sidebar.

**Mute** stops your microphone reaching the server. **Deafen** mutes everything incoming.

Both can be bound to a key, along with push-to-talk. See [Settings](/wiki/player/settings/).

## Adjust another player

Each player tile has its own controls. Mute someone outright, or set their volume between 0% and 150%.

Set the volume while they are inside 24 blocks. That is their unattenuated level, and distance falloff applies on top of it.

Muting someone affects only what **you** hear. They can still hear you.

Adjustments stay after the player walks away or logs off. To find someone no longer on screen, or to undo a mute, open **Settings → Players**. It lists everyone you have heard and has a reset for all of them.
