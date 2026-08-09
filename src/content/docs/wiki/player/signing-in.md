---
title: Signing in
description: Connect the BVC app to a server with your Xbox account.
sidebar:
  label: Signing in
  order: 2
---

BVC signs you in with your Xbox account. That is how the server knows which in-game player you are.

## Join the Minecraft server first

Join the Minecraft server itself at least once before your first sign-in. BVC identifies you from the position data the server relays.

Your operator can add you ahead of time instead:

```
bedrock-voice-chat-server user add --player <gamertag> --game minecraft
```

## Your login URL

The login URL is the address of the **BVC server**. Not your Minecraft server, not your Aternos address, not your Realm, not the IP you type into Minecraft.

It includes `https://` and the full hostname:

```
https://example.bedrockvc.stream
```

Your operator gives you this address. There is no way to find it from the app.

<div class="shot"><span>BVC login window with the server URL field filled in</span></div>

## Sign in

1. Enter the URL.
2. Sign in on the Microsoft page that opens. Use the account tied to the gamertag you play as on that server.
3. Wait while BVC validates the session and connects.

You land on the dashboard. Headphones on, go play.

<div class="shot"><span>BVC dashboard immediately after a successful sign-in</span></div>

## Add another server

One identity, many servers. Every server uses the same Xbox account and has its own address. Add each one separately.

## If sign-in fails

**`AUTH02`, or a rejected login.** BVC is deny-by-default. Your operator has to add you first.

**The URL is refused.** Check you are using the BVC server address and not the Minecraft one. This is the most common mistake.

**`DNS01`.** The address did not resolve. Check for a typo.

**It hangs or times out.** The BVC server may be down. Your operator can check with `curl https://<their-server>/api/config`.

More in [troubleshooting](/wiki/player/troubleshooting/).
