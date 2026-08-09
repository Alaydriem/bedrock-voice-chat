---
title: Audio library
description: Upload and manage the audio clips that BVC discs play.
sidebar:
  label: Audio library
  order: 7
---

The audio library holds clips stored on the BVC server. Anyone can bind a stored clip to a disc and play it through a [jukebox](/wiki/player/using-the-jukebox/).

Open **Settings → Audio library**.

<div class="shot"><span>Audio Library settings page with several uploaded clips</span></div>

## Permissions

| To | You need |
|---|---|
| Play a clip | Nothing. Anyone can make a disc. |
| Upload | `audio_upload` |
| Delete | `audio_delete` |

Operators set these server-wide and can override them per player. If the upload button does nothing, ask your operator for the permission. See [players and permissions](/wiki/server/players-and-permissions/).

## Upload a clip

Upload a clip and the server gives it an ID. That ID is what `/bvc:disc <audio_id>` takes.

Clips live on the server. Everyone on the server shares one library, and a clip you upload is a clip everyone can play.

## Limits

`max_concurrent_per_uuid` caps how many clips one player can have playing at once. Default is 5.

Storage is the operator's disk. There is no per-player quota.

## For operators

Clip storage path and the concurrency cap are the `audio` block in [config.hcl](/wiki/reference/configuration/#audio). Permission defaults are the `permissions` block.

On a public server, set `audio_upload = false` as the default and grant it per player. An open default lets anyone fill the disk.
