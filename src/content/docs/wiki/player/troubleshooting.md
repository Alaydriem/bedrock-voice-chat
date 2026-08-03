---
title: Troubleshooting
description: What to check when BVC is not working.
sidebar:
  label: Troubleshooting
  order: 10
---

## I cannot sign in

**Rejected or "not whitelisted".** The operator has to add you. BVC is deny-by-default and nothing in the app gets around it.

**The URL is refused.** You are probably using the Minecraft server address. The login URL is the **BVC server** — see [signing in](/wiki/player/signing-in/).

**It hangs.** The BVC server may be down. Ask your operator to check `curl https://<their-server>/api/config`.

## I signed in but hear nobody

Work down this list in order.

**Is anyone else actually using it?** Everyone needs the app running and signed in. BVC cannot make a player audible if they are not connected.

**Are you within range?** Default is 24 blocks for full volume, fading to nothing at 48. Stand next to someone and try again.

**Are you deafened?** Check the deafen button at the bottom left. Check per-player mutes on the tiles too.

**Is your output device right?** Settings → Audio. If BVC is playing to a device you are not wearing, it looks identical to silence.

**Did you connect the way your server needs?** On Realms and Aternos you must go through Realms Connect or Proxy Connect. Connecting directly to the server address skips BVC entirely and everything looks normal except nobody can hear anyone.

## Nobody can hear me

**Muted?** Bottom left.

**Push-to-talk bound and not held?** If you set a PTT key, that is the only time you transmit. Settings → Keybinds.

**Right input device?** Settings → Audio. Check the level meter moves when you speak.

**Noise gate too aggressive?** If you are quiet or far from the mic, the gate may never open. Lower the open threshold, or turn the gate off to test. See [Settings](/wiki/player/settings/).

## Audio cuts in and out

Usually the noise gate closing between words. Raise the hold time.

If it is not the gate, it is likely the network. QUIC needs UDP; some networks throttle or block it. Your operator can publish a second QUIC port for clients on restrictive networks.

## It worked yesterday

**Did Minecraft update?** On Realms or Aternos an update breaks voice until a matching BVC build ships. See [version support](/wiki/platforms/version-support/).

**Did you update BVC?** Server and Addon versions have to be compatible. Ask your operator whether the server is current.

## Still stuck

Grab the specifics before asking — the server address you are using, your platform, whether anyone else on the server has it working, and what you have already ruled out from this page.

[Discord](https://discord.gg/CdtchD5zxr) or a [GitHub issue](https://github.com/Alaydriem/bedrock-voice-chat/issues).
