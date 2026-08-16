---
title: Troubleshooting
description: What to check when BVC is not working.
sidebar:
  label: Troubleshooting
  order: 10
---

## I cannot sign in

**`AUTH02`, or "not whitelisted".** Your operator has to add you. BVC is deny-by-default.

**The URL is refused.** You are probably using the Minecraft server address. The login URL is the **BVC server**. See [signing in](/wiki/player/signing-in/).

**It hangs.** The BVC server may be down. Ask your operator to run `curl https://<their-server>/api/config`.

## I signed in but hear nobody

Work down this list in order.

1. **Is anyone else using it?** Everyone needs the app running and signed in.
2. **Are you in range?** Default is 24 blocks for full volume, fading to nothing at 48. Stand next to someone.
3. **Are you deafened?** Check the deafen button at the bottom left, and the per-player mutes on the tiles.
4. **Is your output device right?** Open Settings → Audio and select **Test playback**. If you hear no chime, pick a different output.
5. **Did you connect the way your server needs?** On Realms and Aternos you must go through Bedrock Voice Chat Connect. Connecting to the server address directly skips BVC.

## Nobody can hear me

**Muted?** Bottom left.

**Push-to-talk bound and not held?** With a PTT key set, that is the only time you transmit. Open Settings → Keybinds.

**Right input device?** Open Settings → Audio and select **Test my microphone**. Speak for a few seconds. If the mark stays flat, pick a different input.

**Noise gate too aggressive?** If you are quiet or far from the mic, the gate may never open. Lower the open threshold, or turn the gate off to test. See [Settings](/wiki/player/settings/).

## Audio cuts in and out

Usually the noise gate closing between words. Raise the hold time.

If it is not the gate, it is the network. QUIC needs UDP, and some networks throttle or block it. Your operator can publish a second QUIC port.

## The record button does nothing

**Are you on a phone?** Recording is desktop only.

**Has your server turned it off?** Operators can forbid recording. The app then refuses to arm from the button, `/bvc:record`, and the Stream Deck. Ask your operator. See [recording](/wiki/creator/recording/).

## I cannot export an old recording

Export needs the recording format version to match your app, and a BVC update can change it. Older sessions stay on disk and stay listed. Export is refused.

Export what you want to keep before you update. See [recording](/wiki/creator/recording/#format-version).

## It worked yesterday

**Did Minecraft update?** On Realms or Aternos an update breaks voice until a matching BVC build ships. See [version support](/wiki/platforms/version-support/).

**Did you update BVC?** `VER01` means your app is behind the server. `VER02` means the server is behind your app. The versions are in Settings → About.

## Error codes

Quote the code when you ask for help.

| Code | Means | Fix |
|---|---|---|
| `AUTH02` | The server has not granted you access | Join the Minecraft server in game once, or ask the operator to add you |
| `AUTH01` | The server could not verify your identity | Sign in again |
| `CONN01` | No answer from the server | Ask your operator |
| `QUIC01` | HTTP got through, voice did not | Something blocks UDP on the voice port. Check your network, then ask your operator |
| `DNS01` | The address did not resolve | Check for a typo |
| `VER01` | Your app is older than the server | Update the app |
| `VER02` | The server is older than your app | Ask your operator to update |
| `AUDI01` `AUDI02` `AUDI03` | An input or output device is missing or unusable | Open Settings → Audio |
| `AGE01` | An adult needs to finish setting up this account | See [parents and age settings](/wiki/player/parents-and-age/) |

`AUTH02` is not a fault. The server has not registered you yet.

## Still stuck

Before asking, collect the server address you are using, your platform, the error code, whether anyone else on the server has it working, and what you already ruled out.

[Discord](https://discord.gg/CdtchD5zxr) or a [GitHub issue](https://github.com/Alaydriem/bedrock-voice-chat/issues).
