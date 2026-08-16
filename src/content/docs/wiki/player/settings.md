---
title: Settings
description: Audio devices, noise gate, keybinds, and the rest of the settings panes.
sidebar:
  label: Settings
  order: 8
---

Bedrock Voice Chat has a variety of different settings that enable you to customize and tailor your experience.

## Audio

<div class="shot"><span>Audio settings page showing input and output device selection</span></div>

### Devices

BVC defaults to using your platforms default communication device and speaker, with dropdowns so that you can use a specific input and output device.

Windows supports both the standard Windows audio drivers and ASIO devices.

### Test your devices

- **Test my microphone.** Speak for a few seconds. The mark fills as BVC hears you. If it stays flat, select a different input device.
- **Test playback.** Plays a chime through the output device BVC uses.

### Voice mode

BVC has two voice modes: **Open mic** transmits whenever the noise gate opens. **Push to talk** transmits only while your key is held. Bind the key under Keybinds.

### Mute cues

BVC plays a short tone when you mute or deafen, and another when you turn either off. On by default.

### Spatial panning

How hard voices are pushed left and right by where the speaker stands. At 0% everyone is centred. Distance still governs volume.

### Jukeboxes

One volume and one mute for every jukebox in the world. Voices are unaffected. See [using the jukebox](/wiki/player/using-the-jukebox/).

### Noise gate

BVC has a built-in noise gate that behaves like OBS's. The defaults handle fans, keyboards, and mouse clicks.

Skip it if you already run a VST chain or system-level noise suppression. Two gates fighting each other sound worse than either alone.

| Control | What it does |
|---|---|
| Open Threshold | Level your voice has to exceed for the gate to open. |
| Close Threshold | Level it has to fall below for the gate to close. Set below the open threshold. |
| Attack Rate | How fast the gate opens once you cross the threshold. |
| Hold Time | How long it stays open after you drop below. Stops it chopping between words. |
| Release Rate | How fast it closes after the hold expires. |

## Keybinds

Global shortcuts that work while Minecraft has focus.

| Action | Notes |
|---|---|
| Toggle Mute | |
| Toggle Deafen | |
| Toggle Recording | Desktop only. |
| Push to Talk | Transmits only while held. |

## Players

Everyone you have heard, with a volume and a mute for each. Changes stick after they walk away, and only you hear the difference.

Search when the list gets long. **Reset everybody** returns every player to full volume and unmutes them all.

## Recordings

Review past sessions, rename them, and export players to separate files as BWAV or MP4/Opus. See [Recording](/wiki/creator/recording/).

<div class="shot"><span>Recordings settings page listing recent sessions</span></div>

## Language

Under About. Pick a language, or **Match my system**. Untranslated text stays in English.

## The rest

| Pane | What it does |
|---|---|
| Account | Your signed-in identity and connected servers. |
| Audio library | Upload and manage audio clips. Needs the `audio_upload` permission. See [audio library](/wiki/player/audio-library/). |
| WebSocket server | Local control API, used by the [Stream Deck plugin](/wiki/creator/stream-deck/). |
| About | Version, build information, language, and diagnostics. |

Under **Minecraft Bedrock**:

| Pane | What it does |
|---|---|
| Voice Chat Connect | Connect to a Realm, or through Aternos and other no-net hosts. See [Realms](/wiki/platforms/realms/) and [Aternos](/wiki/platforms/aternos/). |

Realms and no-net hosts share one pane. Pick the world from the list.

Recordings and Keybinds are desktop only.
