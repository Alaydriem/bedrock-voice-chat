---
title: Settings
description: Audio devices, noise gate, keybinds, and the rest of the settings pages.
sidebar:
  label: Settings
  order: 8
---

The gear icon at the bottom left of the BVC menu.

## Audio

<div class="shot"><span>Audio settings page showing input and output device selection</span></div>

### Devices

BVC uses your platform's default communication device and speaker unless you pick otherwise. The dropdowns let you choose any specific microphone and output.

On Windows both the standard Windows audio drivers and ASIO are supported.

All input is mixed to mono regardless of the device. Proximity audio positions a mono source in 3D space, so a stereo microphone buys you nothing here.

### Noise gate

BVC has a built-in noise gate that behaves like OBS's. The defaults handle the usual background — fans, keyboard, mouse clicks — well enough for most people. The dials are there if you want to tune it.

Skip it if you already run a VST chain or system-level noise suppression. Two gates fighting each other sounds worse than either alone.

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

## Recordings

Review past sessions and export players to separate files, as BWAV or MP4/Opus. See [Recording](/wiki/creator/recording/).

<div class="shot"><span>Recordings settings page listing recent sessions</span></div>

## The rest

| Page | What it does |
|---|---|
| Account | Your signed-in identity and connected servers. |
| Audio Library | Upload and manage clips. Needs the `audio_upload` permission. See [audio library](/wiki/player/audio-library/). |
| Proxy Connect | Connect through Aternos and other no-net hosts. See [Aternos](/wiki/platforms/aternos/). |
| Realms Connect | Connect to a Realm. See [Realms](/wiki/platforms/realms/). |
| WebSocket | Local control API, used by the [Stream Deck plugin](/wiki/creator/stream-deck/). |
| About | Version and build information. |
