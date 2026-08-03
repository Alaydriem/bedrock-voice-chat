---
title: Signing in
description: Connect the BVC app to a server with your Xbox account.
sidebar:
  label: Signing in
  order: 2
---

BVC authenticates against your Xbox account. That is how the server knows which in-game player you are, and it is what secures traffic between you and everyone else.

## Connect to the server first

Before your first sign-in, join the Minecraft server itself at least once. BVC learns who you are from the position data the server relays, so it has to have seen you in-game.

Your operator can skip this by adding you ahead of time:

```
bedrock-voice-chat-server user add --player <gamertag> --game minecraft
```

## Your login URL

The login URL is the address of the **BVC server**, not your Minecraft server. Not your Aternos address, not your Realm, not the IP you type into Minecraft.

It includes `https://` and the full hostname:

```
https://example.bedrockvc.stream
```

Your operator sets this up and gives it to you. If you do not have it, ask them — there is no way to discover it from the app.

<div class="shot"><span>BVC login window with the server URL field filled in</span></div>

## Signing in

Enter the URL and you are redirected to a Microsoft sign-in page. Use the account tied to the Xbox gamertag you play as on that server.

After signing in you are sent back to BVC, which validates the session, connects, and does some first-run setup. It takes a moment.

Then you land on the dashboard. Headphones on, go play.

<div class="shot"><span>BVC dashboard immediately after a successful sign-in</span></div>

## Multiple servers

BVC works like Discord: one identity, many servers. Every server uses the same Xbox account but has its own address, and you add each one separately.

## If sign-in fails

**"Not whitelisted" or a rejected login.** BVC is deny-by-default. Your operator has to add you before you can connect. Nothing in the app can override that.

**The URL is refused.** Check you are using the BVC server address and not the Minecraft one. This is the single most common mistake.

**It hangs or times out.** The BVC server may be down or unreachable. Your operator can confirm with `curl https://<their-server>/api/config`.

More in [troubleshooting](/wiki/player/troubleshooting/).
