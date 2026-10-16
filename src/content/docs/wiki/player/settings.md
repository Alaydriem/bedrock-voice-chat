---
title: Settings
description: Audio devices, noise gate, keybinds, and the rest of the settings panes.
sidebar:
  label: Settings
  order: 8
---

Bedrock Voice Chat has settings for audio devices, voice mode, the noise gate, keybinds, and more.

## Audio settings

<div class="window"><img src="/assets/wiki/settings-audio.jpg" alt="Audio settings page showing input and output device selection" /></div>

### Devices

By default, BVC uses the default communication device and speaker of your computer or phone. To use a different microphone or speaker, select it from the list.

On Windows, BVC works with standard audio devices and with ASIO devices (professional audio interfaces).

### Test your devices

- **Test my microphone.** Speak for a few seconds. The bar fills when BVC hears you. If the bar does not move, select a different input device.
- **Test playback.** BVC plays a chime through the output device.

### Voice mode

BVC has two voice modes:

- **Open mic** sends your voice when you speak loudly enough to open the noise gate.
- **Push to talk** sends your voice only while you hold the push-to-talk key. Set the key under [Keybinds](#keybinds).

### Mute cues

BVC plays a short tone when you mute or deafen, and a different tone when you unmute or undeafen. This setting is on by default.

### Spatial panning

Spatial panning sets how much BVC moves a voice to the left or right, based on where the player stands. At 0%, all voices are in the centre. The volume still changes with distance.

### Jukeboxes

This setting has one volume control and one mute for all jukeboxes in the world. It does not change the volume of voices. See [using the jukebox](/wiki/player/using-the-jukebox/).

<div class="window"><img src="/assets/wiki/settings-audio-more.jpg" alt="Audio settings page showing voice chat sounds, jukebox volume, and the noise gate toggle" /></div>

### Noise gate

BVC has a noise gate that works like the noise gate in OBS. The default settings block the noise from fans, keyboards, and mouse clicks.

If you already use other noise suppression (for example, VST plug-ins or the noise suppression of your operating system), turn off this noise gate. Two noise gates together make your voice sound worse.

| Control | Effect |
|---|---|
| Open Threshold | The level that your voice must go above to open the gate. |
| Close Threshold | The level that your voice must go below to close the gate. Set it below the open threshold. |
| Attack Rate | How fast the gate opens after your voice goes above the threshold. |
| Hold Time | How long the gate stays open after your voice goes below the threshold. This stops the gate from cutting off your voice between words. |
| Release Rate | How fast the gate closes after the hold time ends. |

## Keybinds

Keybinds are keyboard shortcuts. They work while Minecraft is the active window.

<div class="window"><img src="/assets/wiki/settings-keybinds.jpg" alt="Keybinds settings page listing mute, deafen, recording, and push to talk" /></div>

| Action | Notes |
|---|---|
| Toggle Mute | |
| Toggle Deafen | |
| Toggle Recording | Desktop only. |
| Push to Talk | Sends your voice only while you hold the key. |

## Player audio levels

**Player audio levels** lists all players that you have heard. Each player has a volume control and a mute. Your changes stay after a player walks away. Only you hear the changes.

Use the search box to find a player in a long list. **Reset everybody** sets all players back to full volume and unmutes them.

<div class="window"><img src="/assets/wiki/settings-players.jpg" alt="Player audio levels page listing players with a volume slider and mute for each" /></div>

## Recordings

See your recorded sessions, rename them, and export the audio of each player to a separate file (BWAV or MP4/Opus). See [Recording](/wiki/creator/recording/).

<div class="window"><img src="/assets/wiki/settings-recordings.jpg" alt="Recordings settings page listing recent sessions" /></div>

## Language

The language setting is on the **About** page. Select a language, or **Match my system**. Text that is not translated stays in English.

<div class="window"><img src="/assets/wiki/settings-about.jpg" alt="About page showing build information, display language, and diagnostics" /></div>

## Other settings

<div class="window"><img src="/assets/wiki/settings-account.jpg" alt="Account page showing the signed-in identity and linked accounts" /></div>

| Page | Contents |
|---|---|
| Account | Your signed-in identity and connected servers. |
| Audio library | Upload and manage audio clips. Needs the `audio_upload` permission. See [audio library](/wiki/player/audio-library/). |
| WebSocket server | Lets other apps on your computer control BVC. The [Stream Deck plugin](/wiki/creator/stream-deck/) uses it. |
| About | Version, build information, language, and diagnostics. |

Under **Minecraft Bedrock**:

| Page | Contents |
|---|---|
| Voice Chat Connect | Connect to a Realm, or to Aternos and other hosts that use the no-net Addon. Select the world from the list. See [Realms](/wiki/platforms/realms/) and [Aternos](/wiki/platforms/aternos/). |

Recordings and Keybinds are available only on desktop.
