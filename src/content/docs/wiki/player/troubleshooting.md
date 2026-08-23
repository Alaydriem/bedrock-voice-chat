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
| `AUTH03` | This device could not save your sign-in to its secure storage | Try again. If it keeps happening, restart the device |
| `AUTH04` | This device has no unlocked keyring to save your sign-in in | See [secure storage](#secure-storage) |
| `CONN01` | No answer from the server | Ask your operator |
| `QUIC01` | HTTP got through, voice did not | Something blocks UDP on the voice port. Check your network, then ask your operator |
| `DNS01` | The address did not resolve | Check for a typo |
| `CERT01` | The server's certificate does not match the one this device was issued | Sign in again. Your saved sign-in for that server was removed. If it recurs, ask your operator to fix their certificate |
| `CERT02` | The voice connection presented a certificate this device cannot verify | Ask your operator. Only they can fix it, and your sign-in is kept |
| `VER01` | Your app is older than the server | Update the app |
| `VER02` | The server is older than your app | Ask your operator to update |
| `AUDI01` `AUDI02` `AUDI03` | An input or output device is missing or unusable | Open Settings → Audio |
| `PERM1` | Microphone permission is not granted | Grant microphone access in your system settings, then return to the dashboard |
| `PERM2` | Notification permission is not granted | Grant notifications in your system settings, then return to the dashboard. Mobile needs it for background audio |
| `SERV01` | The background audio service could not start | Mobile only. Check that your system is not restricting the app in the background |
| `AGE01` | An adult needs to finish setting up this account | See [parents and age settings](/wiki/player/parents-and-age/) |

`AUTH02` is not a fault. The server has not registered you yet.

## Secure storage

Bedrock Voice Chat keeps your per-server sign-in in your operating system's credential store. `AUTH03` and `AUTH04` both mean the sign-in worked and the storing did not, so the app cannot complete it.

`AUTH03` is a rejected write, and is usually temporary. Try again, then restart the device.

`AUTH04` means there is no unlocked keyring to write to. This is a Linux setup step.

1. Open **Passwords and Keys** (`seahorse`).
2. Create a password keyring named **Login**.
3. Set it as the default.

If your desktop signs you in automatically, no keyring is unlocked at login. Turn automatic sign-in off and sign in with your password once; that creates and unlocks the keyring.

## Still stuck

Before asking, collect the server address you are using, your platform, the error code, whether anyone else on the server has it working, and what you already ruled out.

[Discord](https://discord.gg/CdtchD5zxr) or a [GitHub issue](https://github.com/Alaydriem/bedrock-voice-chat/issues).
