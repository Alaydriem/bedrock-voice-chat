---
title: "Chat"
description: "Read and send in-game chat from the BVC app, without leaving what you are doing."
source: https://www.bedrockvoicechat.com/wiki/player/chat/
---
Bedrock Voice Chat shows the in-game chat of your server in the app. You can read and reply to chat without leaving what you are doing. The messages that you type go into the game as normal chat messages.

## Send a message

1. Open the chat panel on the dashboard.
2. Type your message.
3. Press Enter.

Your message shows in game as `<yourname> your message`.

<div class="window"><img src="/assets/wiki/chat-open.jpg" alt="BVC Chat Window" /></div>

## Where your message goes

If you are in a Minecraft world, Bedrock Voice Chat sends your message to that world.

If you are signed in to BVC but you are not in a Minecraft world, BVC can show a list of worlds. The list shows when the server has more than one world that you were in before. Select a world from the list. Each world has two labels:

| Label | Meaning |
|---|---|
| **Active** | Players are in the world now. |
| **Available** | Chat to this world works now. |

:::caution
A world can be **Active** but not **Available**. The game server is running, but its chat connection to Bedrock Voice Chat does not work. Your message **will not send** to that world.
:::

## A message does not send

**You changed worlds.** You went to a different world, but the app still had the old world selected. Send the message again.

**The message did not send.** You are not connected, the chat connection does not work, or too many messages are waiting. When the chat panel shows chat as available, send the message again.

## History

Chat history starts when you connect and ends when you close the app. The server and your device do not save it.

## Chat is not available

The message box shows the reason:

- **Not connected.** Sign in to a server first.
- **The server has no BVC Addon or mod.** If you connect directly to a Minecraft server, that server needs the BVC Addon or mod. If you connect to a Realm or with Bedrock Voice Chat Connect, it does not.
- **The chat connection is down.** Wait. The mod connects again automatically.
- **Chat is disabled on this server.** The server owner turned chat off. The chat panel stays on your dashboard, greyed out, and you cannot type in it. You cannot turn it on from the app.

## For server operators

Chat needs no setup on the BVC server. To turn chat off, see [`server.features`](/wiki/reference/configuration/#serverfeatures). For the game server setup, see the [Bedrock Addon](/wiki/server/bedrock-addon/), [Fabric](/wiki/server/java-mod/fabric/#chat-sync) or [PaperMC](/wiki/server/java-mod/papermc/#chat-sync) page.

## Privacy

The BVC server sends chat messages and does not store them. It stores a list of the worlds where it saw each player. Each entry has the player, the world, and the time. BVC uses this list to show the world list.

All players in the world can read your messages.