---
title: Recording
description: Capture a session and export per-player tracks as BWAV or MP4/Opus.
sidebar:
  label: Recording
  order: 2
---

BVC records each player to their own track.

Desktop only: Windows, macOS, and Linux. Mobile devices lack the disk throughput for multi-track capture.

## Start a recording

Press **REC** at the bottom of the sub-sidebar. The indicator turns red.

<div class="shot"><span>The REC button active, showing the red recording indicator</span></div>

Sessions capture position and group data alongside audio. The spatial arrangement is preserved.

Start and stop also work from [in-game](/wiki/player/in-game-commands/) with `/bvc:record <on|off>`, and from a [Stream Deck](/wiki/creator/stream-deck/).

## Server policy

An operator can forbid recording for everyone connected to their server:

```hcl
voice {
    recording {
        enabled = false
    }
}
```

Recording is permitted by default. A server that predates this setting reads as permitted.

This is a client policy, not a server boundary. The recording is written on the player's own machine and never reaches the server. It stops the REC button, `/bvc:record`, and the WebSocket `record` action in the stock client. It does not stop a modified client or a screen recorder.

See [configuration](/wiki/reference/configuration/#voicerecording).

## Manage recordings

Open **Settings → Recordings**.

<div class="shot"><span>Recordings page listing sessions with their participants</span></div>

Review sessions, see participants, rename, delete, and export.

Sessions are listed by timestamp until you name one. Clearing a name restores the timestamp.

Export writes every participant as a separate file. Expand the participant dropdown to pick a subset.

Each file is named for the player. Somebody signed in on two devices records as two tracks.

BVC opens the output folder when an export finishes:

```
%localappdata%/com.alaydriem.bvc.client/recordings/<session_uuid>/renders
```

## Format version

A session records the format version it was written with. Export requires an exact match with the version your app is running.

:::caution
**A BVC update can make older sessions unexportable.** They stay on disk and stay listed. Export is refused, and downgrading the app is the only way back.

Export what you want before you update. The format version changed in this release. Every session recorded before it is affected.
:::

## Export formats

| Format | Notes |
|---|---|
| **BWAV** (`.wav`) | Broadcast WAV with timecode. Raw PCM. Large, lossless, accepted by every editor. |
| **MP4 / Opus** | Opus in an MP4 container, with timecode. Much smaller. |

Both carry timecode. Tracks line up on an editor timeline without manual syncing.

## Disk

BWAV is raw PCM. Roughly 5–15 MB per player per 30 seconds. A two-hour session with six participants renders 7–21 GB.

To manage it:

- Export to MP4/Opus unless your editor needs PCM.
- Break long sessions into shorter recordings.
- Delete old renders. Nothing cleans them up automatically.

## Consent

Anyone in a group can record everyone in that group, and nobody is notified. Proximity audio is the same.

Tell people you are recording. Whether that is a courtesy or a legal requirement depends on where you and they are. BVC does not enforce either.
