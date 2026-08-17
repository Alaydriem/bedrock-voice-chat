---
title: Chat
description: Read and send in-game chat from the BVC app, without leaving what you are doing.
sidebar:
  label: Chat
  order: 5
---

Bedrock Voice Chat mirrors your server's in-game chat into the app, letting you read and reply without leaving what you are doing. What you type goes back into the game as an ordinary chat message.

## Send a message

1. Open the chat dock on the dashboard.
2. Type.
3. Press enter.

Your message appears in game as `<yourname> your message`.

<div class="shot"><span>BVC Chat Window</span></div>

## Where your message goes

If you are standing in a world, Bedrock Voice Chat sends your message there and does not ask.

You get a picker when you are signed in, not in game, and the server carries more than one world you have been in. Each world in the list shows two labels:

| Label | Meaning |
|---|---|
| **Active** | People are in it right now |
| **Available** | Its chat relay is up |

<div class="shot"><span>The chat target picker listing worlds with their Active and Available labels</span></div>

:::caution
A world can be **Active** without being **Available**. The game server is running, but its chat link to Bedrock Voice Chat is down, and your message **will not send** there.
:::

## If a message does not send

**You moved.** You changed worlds while the app still held the old one. Send it again.

**It failed.** No session, a dead link, or a full queue. Send it again once the dock shows chat as available.

## History

Chat history starts when you connect and ends when you close the app. Nothing is stored on the server or on your device.

## If chat is not available

The composer says why.

- **Not connected.** Sign in to a server first.
- **The server has no BVC Addon or mod.** Server mode needs one. Proxy and Realms sessions do not.
- **The chat relay is down.** The mod reconnects on its own.
- **Chat is disabled on this server.** The server owner turned chat off. The dock stays on your dashboard, greyed out, and you cannot type in it. Nothing you do in the app turns it back on.

## For server operators

Nothing to enable on the BVC server. Chat works wherever position relay does.

To turn chat off, set `chat = false` in the `server.features` block of `config.hcl`, or set `BVC_CHAT=false`. Restart the server to apply it. See [config.hcl](/wiki/reference/configuration/).

**Fabric and PaperMC** — set `enforce-secure-profile=false` in `server.properties`. Chat sync does not work with it enabled. Everything else is automatic in both external and embedded mode. The app shows your primary world's name.

**Bedrock Dedicated Server** — add `bvc_world_name` to `config/default/variables.json`. BDS scripting cannot read your level name.

```json
{
    "bvc_world_name": "My SMP"
}
```

Without it, the world is listed as "Minecraft world". See the [Bedrock Addon](/wiki/server/bedrock-addon/).

## Privacy

The BVC server relays messages and never stores them. It records which worlds each player has been seen in, to populate the picker. That is a player, a world, and a timestamp.

Anyone in the world reads what you send.
