---
title: "Troubleshooting"
description: "What to check when BVC is not working."
source: https://www.bedrockvoicechat.com/wiki/player/troubleshooting/
---
## I cannot sign in

**`AUTH02`, or "not whitelisted".** Join the Minecraft server in game one time, then sign in again. If you still cannot sign in, ask your server operator to add you.

**BVC does not accept the URL.** You are probably using the Minecraft server address. The server URL is the address of the **BVC server**. See [signing in](/wiki/player/signing-in/).

**Sign-in does not finish.** The BVC server may be down. Ask your server operator to check that the BVC server is running.

## I signed in but hear nobody

Do these checks in order.

1. **Do other players use BVC?** All players need the app open and signed in.
2. **Are you near other players?** By default, you hear players at full volume up to 24 blocks away. At 48 blocks, you hear nothing. Stand next to a player.
3. **Are you deafened?** Check the deafen button at the bottom left. Also check the mute button on each player in the player list.
4. **Is your output device correct?** Open **Settings → Audio settings** and select **Test playback**. If you do not hear a chime, select a different output device.
5. **Do you play on Realms or Aternos?** Connect with **Voice Chat Connect** in the app. If you connect to the server address directly, BVC does not work.

## Nobody can hear me

**Are you muted?** Check the mute button at the bottom left.

**Do you use push-to-talk?** If you set a push-to-talk key, BVC sends your voice only while you hold that key. Open **Settings → Keybinds**.

**Is your input device correct?** Open **Settings → Audio settings** and select **Test my microphone**. Speak for a few seconds. If the bar does not move, select a different input device.

**Is the noise gate too strong?** If you speak quietly or are far from the microphone, the gate may never open. Lower the open threshold, or turn the gate off to test. See [Settings](/wiki/player/settings/).

## Audio cuts in and out

Usually, the noise gate closes between your words. Increase the hold time.

If the noise gate is not the cause, the cause is probably your network. Some networks slow down or block the connection that BVC uses for voice. Try a different network, then ask your server operator.

## The record button does nothing

**Are you on a phone?** Recording is available only on desktop.

**Did your server turn recording off?** Server operators can turn off recording. The app then does not start a recording from the button, from `/bvc:record`, or from the Stream Deck. Ask your server operator. See [recording](/wiki/creator/recording/).

## I cannot export an old recording

You can export a recording only if it uses the same recording format as your app. A BVC update can change this format. After the update, old recordings stay on your device and in the list, but you cannot export them.

Export the recordings that you want to keep before you update. See [recording](/wiki/creator/recording/#format-version).

## It worked yesterday

**Did Minecraft update?** On Realms or Aternos, a Minecraft update stops voice chat until a BVC version for that Minecraft version is available. See [version support](/wiki/platforms/version-support/).

**Did you update BVC?** `VER01` means that your app is older than the server. `VER02` means that the server is older than your app. You can see the versions in **Settings → About**.

## Error codes

Give the code when you ask for help.

| Code | Meaning | Fix |
|---|---|---|
| `AUTH02` | The server does not know you yet. | Join the Minecraft server in game one time, or ask the operator to add you. |
| `AUTH01` | The server could not confirm who you are. | Sign in again. |
| `AUTH03` | This device could not save your sign-in. | Try again. If it happens again, restart the device. |
| `AUTH04` | This device has no unlocked keyring (password storage) for your sign-in. | See [secure storage](#secure-storage). |
| `CONN01` | BVC cannot connect to the server. | Ask your server operator. |
| `QUIC01` | BVC connected to the server, but the voice connection failed. | Your network may block voice. Try a different network, then ask your server operator. |
| `DNS01` | BVC could not find the server address. | Check the address for a typing error. |
| `CERT01` | The server's security certificate changed after you signed in. | Sign in again. BVC deleted your saved sign-in for this server. If this happens again, ask your server operator. |
| `CERT02` | BVC cannot confirm the security of the voice connection. | Ask your server operator. Only the operator can fix this. Your sign-in is kept. |
| `VER01` | Your app is older than the server. | Update the app. |
| `VER02` | The server is older than your app. | Ask your server operator to update. |
| `AUDI01` `AUDI02` `AUDI03` | An input or output device is missing or does not work. | Open **Settings → Audio settings**. |
| `PERM1` | BVC does not have permission to use the microphone. | Allow microphone access in your system settings, then go back to the dashboard. |
| `PERM2` | BVC does not have permission to show notifications. | Allow notifications in your system settings, then go back to the dashboard. On mobile, BVC needs this permission to play audio in the background. |
| `SERV01` | The background audio service could not start. | Mobile only. Make sure that your system does not limit the app in the background. |
| `AGE01` | An adult must finish the setup of this account. | See [parents and age settings](/wiki/player/parents-and-age/). |

`AUTH02` does not mean that something is broken. The server has not added you yet.

## Secure storage

Bedrock Voice Chat saves your sign-in for each server in the password storage of your operating system. `AUTH03` and `AUTH04` mean that you signed in correctly, but BVC could not save the sign-in. BVC cannot finish the sign-in without it.

`AUTH03` means that your operating system did not let BVC save the sign-in. This problem usually stops by itself. Try again. If it continues, restart the device.

`AUTH04` means that your computer has no unlocked keyring (password storage). This error occurs on Linux. To fix it:

1. Open **Passwords and Keys** (`seahorse`).
2. Create a password keyring with the name **Login**.
3. Set it as the default keyring.

If your computer logs you in automatically when it starts, no keyring is unlocked. Turn off automatic login. Then log in to your computer with your password one time. This creates and unlocks the keyring.

## Still stuck

Before you ask for help, collect this information:

- The BVC server address.
- Your device and operating system.
- The error code.
- If BVC works for other players on the server.
- The fixes that you already tried.

Ask on [Discord](https://discord.gg/CdtchD5zxr) or open a [GitHub issue](https://github.com/Alaydriem/bedrock-voice-chat/issues).