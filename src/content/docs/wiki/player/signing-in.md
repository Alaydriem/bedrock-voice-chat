---
title: Signing in
description: Connect the BVC app to a server with your Xbox account.
sidebar:
  label: Signing in
  order: 2
---

Because Bedrock Voice Chat can perform actions on your behalf and represent you on your server, you must first sign-in to Bedrock Voice Chat using your Microsoft/Xbox Live credentials.

:::tip
Bedrock Voice Chat only stores your gamertag and profile picture on servers that you connect to. No other data is collected, and no data is centrally collected.
:::

## Join the Minecraft server first

Make sure before you sign into Bedrock Voice Chat, you login to your Minecraft instance _first_. Bedrock Voice Chat works on your servers allow-list, and builds its permissions list from your Minecraft server.

:::caution
If you are a Realms or Aternos player using the no-net addon, you server operator **MUST** manually whitelist you on Bedrock Voice Chat Server **FIRST** before you can login.
:::

## Your login URL

The login URL is the address of the **BVC server**. Not your Minecraft server, not your Aternos address, not your Realm, not the IP you type into Minecraft.

It includes `https://` and the full hostname:

```
https://example.bedrockvc.stream
```

Your operator gives you this address.

<div class="shot"><span>BVC login window with the server URL field filled in</span></div>

## Sign in

1. Enter the URL.
2. Sign in on the Microsoft page that opens. Use the account tied to the gamertag you play as on that server.
3. Wait while BVC validates the session and connects.

You land on the dashboard. Headphones on, go play.

<div class="shot"><span>BVC dashboard immediately after a successful sign-in</span></div>
