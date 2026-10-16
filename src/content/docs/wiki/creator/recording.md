---
title: Recording
description: Capture a session and export per-player tracks as BWAV or MP4/Opus.
sidebar:
  label: Recording
  order: 2
---

The recording feature of Bedrock Voice Chat records a session with each player on a separate track. You can mix and edit each voice separately.

:::note
Recording is available only on Windows, macOS and Linux. Mobile devices cannot write multi-track audio to storage fast enough.
:::

## Start a recording

Press **REC** at the bottom of the secondary sidebar. The indicator turns red.

A session records player positions and group data with the audio. The export can put each voice at the position where the player stood.

You can also start and stop a recording [in game](/wiki/player/in-game-commands/) with `/bvc:record <on|off>`, or from a [Stream Deck](/wiki/creator/stream-deck/).

## Server policy

An operator can stop all players on their server from recording:

```hcl
voice {
    recording {
        enabled = false
    }
}
```

Recording is allowed by default. On a server older than 1.0.0-beta.21, recording is allowed.

:::caution
The client applies this setting. The server cannot enforce it. Recordings are saved on the player's own computer and never go to the server.

When `enabled = false`, the standard client disables the REC button, `/bvc:record`, and the WebSocket `record` action. A modified client or a screen recorder can still record.
:::

See [configuration](/wiki/reference/configuration/#voicerecording).

## Manage recordings

Open **Settings → Recordings**.

<div class="window"><img src="/assets/wiki/settings-recordings.jpg" alt="Recordings page listing sessions with their participants" /></div>

**Recordings** lists your sessions and their participants. You can rename, delete and export each session.

The list shows each session by its timestamp until you give it a name. If you clear the name, the list shows the timestamp again.

The export writes each track to a separate file. To export only some tracks, expand the track list and select them.

| Track | Contents |
|---|---|
| Your own | Your microphone. |
| Player | The voice of one other player. |
| Jukebox | Music from a jukebox. |

Each player track has the player's name. A player who is signed in on two devices records as two tracks.

When an export is complete, BVC opens the output folder:

| System | Folder |
|---|---|
| Windows | `%localappdata%/com.alaydriem.bvc.client/recordings/<session_uuid>/renders` |
| macOS | `~/Library/Application Support/com.alaydriem.bvc.client/recordings/<session_uuid>/renders` |
| Linux | `~/.local/share/com.alaydriem.bvc.client/recordings/<session_uuid>/renders` |

## Spatial export

**Mix in the spatial positions** puts each voice at the position where the player stood, and writes each track in stereo. It is on by default.

Turn it off to write each track in mono at the centre, directly from the recording, with no re-encoding. Use this if you want to position the voices yourself.

A spatial export needs two values that are not in the recording: the server's spatial audio settings and your panning intensity. BVC uses the values from the live session. If there is no live session, it uses the last known values, and then the defaults. The export log shows which values it used.

If the export uses the defaults, and the server uses a different `broadcast_range`, the volume of each voice over distance is wrong in the export. You cannot hear this error easily. If an export sounds wrong, read the export log.

See [`voice.spatial_audio`](/wiki/reference/configuration/#voicespatial_audio).

## Format version

Each session stores the format version that it was recorded with. To export a session, this version must be the same as the version of your app.

:::caution
**A BVC update can stop you from exporting older sessions.** The sessions stay on disk and stay in the list, but the export is refused. To export them, install the earlier version of the app.

Export the sessions that you want to keep before you update.
:::

## Export formats

| Format | Notes |
|---|---|
| **BWAV** (`.wav`) | Broadcast WAV with timecode. Uncompressed PCM. Large and lossless. All editors accept it. |
| **MP4 / Opus** | Opus in an MP4 container, with timecode. Much smaller. |

Both formats have timecode. The tracks align on an editor timeline with no manual synchronisation.

## Disk space

BWAV is uncompressed PCM. It uses approximately 5–15 MB for each player for each 30 seconds. A two-hour session with six players makes 7–21 GB.

To use less disk space:

- Export to MP4/Opus, unless your editor needs PCM.
- Split long sessions into shorter recordings.
- Delete old exports. BVC does not delete them automatically.

## Consent

:::danger
Any member of a group can record all members of that group, and BVC does not notify them. A player can also record nearby players, and BVC does not notify them.

Tell people when you record. The law where you and the other players live can require their consent. Bedrock Voice Chat does not ask for consent and does not notify players.
:::
