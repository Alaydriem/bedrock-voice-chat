---
title: Chat
description: Read and send in-game chat from the BVC app, without leaving what you are doing.
sidebar:
  label: Chat
  order: 5
---

BVC mirrors your server's in-game chat into the app. What you type goes back into the game as an ordinary chat message.

## Send a message

1. Open the chat dock on the dashboard.
2. Type.
3. Press enter.

Your message appears in game as `<yourname> your message`.

## History

Chat history starts when you connect and ends when you close the app. Nothing is written to disk, and nothing is stored on the BVC server.

## Where your message goes

If you are standing in a world, your message goes there and you are not asked.

You get a picker when all of these are true: you are signed in, you are not in game, and the BVC server carries more than one world you have been in.

Each world in the list shows two things:

| Label | Meaning |
|---|---|
| **Active** | People are in it right now |
| **Available** | Its chat relay is up |

A world can be active and unavailable. The game server is running, its chat link to BVC is down, and you cannot send there.

## If a message does not send

**You moved.** You changed worlds while the app still held the old one. Send it again.

**It failed.** No session, a dead link, or a full queue. Send it again once the dock shows chat as available.

## Two modes

The app picks the mode. There is nothing to choose.

| Mode | When | What relays chat |
|---|---|---|
| **Server** | The game server runs a BVC Addon or mod | The Addon or mod |
| **Local** | You are connected through Proxy Connect or Realms Connect | Your own proxy session |

Local mode needs nothing from the server operator.

## If chat is not available

The composer says why.

- **Not connected.** Sign in to a server first.
- **The server has no BVC Addon or mod.** Server mode needs one. Proxy and Realms sessions do not.
- **The chat relay is down.** The mod reconnects on its own.

## For server operators

Nothing to enable on the BVC server. Chat works wherever position relay does.

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
