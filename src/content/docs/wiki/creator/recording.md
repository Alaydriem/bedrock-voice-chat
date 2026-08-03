---
title: Recording
description: Capture a session and export per-player tracks as BWAV or MP4/Opus.
sidebar:
  label: Recording
  order: 2
---

BVC records each player to their own track, so you can fix one person in post without touching anyone else.

Desktop only — Windows, macOS, and Linux. Mobile devices lack the disk throughput for multi-track capture.

## Recording

Press **REC** at the bottom of the sub-sidebar. The indicator turns red.

<div class="shot"><span>The REC button active, showing the red recording indicator</span></div>

Sessions capture position and group data alongside audio, so the spatial arrangement is preserved and not just the voices.

You can also start and stop from [in-game](/wiki/player/in-game-commands/) with `/bvc:record <on|off>`, or from a [Stream Deck](/wiki/creator/stream-deck/).

## Managing recordings

Settings → Recordings.

<div class="shot"><span>Recordings page listing sessions with their participants</span></div>

You can review sessions, see participants, delete them, and export.

Exporting writes every participant as a separate file by default. Expand the participant dropdown to pick a subset.

When an export finishes, BVC opens the output folder:

```
%localappdata%/com.alaydriem.bvc.client/recordings/<session_uuid>/renders
```

## Export formats

| Format | Notes |
|---|---|
| **BWAV** (`.wav`) | Broadcast WAV with timecode. Raw PCM — large, lossless, accepted by every editor. |
| **MP4 / Opus** | Opus in an MP4 container, with timecode. Much smaller. |

Both carry timecode, so tracks line up on an editor timeline without manual syncing.

## Disk

BWAV is raw PCM and gets large fast. Roughly 5–15 MB per player per 30 seconds — a two-hour session with six participants can render 7–21 GB.

Three ways to manage it:

- Export to MP4/Opus unless your editor specifically needs PCM.
- Break long sessions into shorter recordings.
- Delete old renders. They are not cleaned up automatically.

## Consent

Everyone in a group can be recorded by anyone in that group, and nobody is notified. Proximity audio is the same.

Tell people you are recording. Whether that is a courtesy or a legal requirement depends on where you and they are, and BVC does not enforce either.
