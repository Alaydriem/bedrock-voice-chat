---
title: Audio library
description: Upload and manage the audio clips that BVC discs play.
sidebar:
  label: Audio library
  order: 7
---

The audio library holds clips stored on the BVC server. Once a clip is there, anyone can bind it to a disc and play it through a [jukebox](/wiki/player/using-the-jukebox/).

Settings → Audio Library.

<div class="shot"><span>Audio Library settings page with several uploaded clips</span></div>

## Permissions

| To | You need |
|---|---|
| Play a clip | Nothing. Anyone can make a disc. |
| Upload | `audio_upload` |
| Delete | `audio_delete` |

Operators set the defaults for these server-wide and can override them per player. If the upload button does nothing, you do not have the permission — ask your operator. See [players and permissions](/wiki/server/players-and-permissions/).

## Uploading

Upload a clip and the server gives it an ID. That ID is what `/bvc:disc <audio_id>` takes.

Clips live on the server, not on your machine, so everyone on the server shares one library. A clip you upload is a clip everyone can play.

## Limits

`max_concurrent_per_uuid` caps how many clips one player can have playing at once. Default is 5.

Storage is the operator's disk. There is no per-player quota, so a server with an open `audio_upload` default is a server where anyone can fill the disk — worth knowing if you run one.

## For operators

Clip storage path and the concurrency cap are the `audio` block in [config.hcl](/wiki/reference/configuration/#audio). Permission defaults are the `permissions` block.

Setting `audio_upload = false` as the default and granting it per player is the usual arrangement on a public server.
