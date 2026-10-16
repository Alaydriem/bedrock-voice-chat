---
title: "Audio library"
description: "Upload and manage the audio clips that BVC discs play."
source: https://www.bedrockvoicechat.com/wiki/player/audio-library/
---
The audio library of Bedrock Voice Chat keeps audio clips on the BVC server. Any player can put a clip on a disc and play it in a [jukebox](/wiki/player/using-the-jukebox/).

Open **Settings → Audio library**.

<div class="window"><img src="/assets/wiki/settings-audio-library.jpg" alt="Audio Library settings page with several uploaded clips" /></div>

## Permissions

| Action | Permission |
|---|---|
| Play a clip | None. Any player can make a disc. |
| Upload | `audio_upload` |
| Delete | `audio_delete` |

The server operator sets these permissions for all players, and can change them for one player. If the upload button does nothing, ask your server operator for the permission.

## Upload a clip

When you upload a clip, the server gives it an ID. To get a disc for the clip, type `/bvc:disc <audio_id>` with that ID.

Clips are stored on the server. All players on the server use the same library. All players can play a clip that you upload.

## For operators

See [players and permissions](/wiki/server/players-and-permissions/) and the [`audio` configuration](/wiki/reference/configuration/#audio). On a public server, set `audio_upload = false` as the default and give it to players you trust.