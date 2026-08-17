---
title: Simple Voice Chat
description: Bridge Simple Voice Chat and Bedrock Voice Chat on one Minecraft server.
sidebar:
  label: Simple Voice Chat
  order: 3
---

Bedrock Voice Chat bridges [Simple Voice Chat](https://modrinth.com/plugin/simple-voice-chat), letting Java players on Simple Voice Chat and players on the BVC app hear each other in the same world, positionally.

Bedrock players cannot install Simple Voice Chat. On a Geyser server the two populations share a world and cannot hear each other without this bridge.

## Requirements

| Component | Where |
|---|---|
| Bedrock Voice Chat Java mod | Fabric or PaperMC. See [Java mod](/wiki/server/java-mod/) |
| Simple Voice Chat | The server-side plugin or mod |
| A BVC server | Embedded in the game server, or external |

The bridge is part of the Java mod. There is no separate download.

:::note
Simple Voice Chat is detected at startup. Installing it on a running server means restarting the game server.
:::

## What crosses the bridge

Simple Voice Chat players continue to hear each other through Simple Voice Chat. That audio never leaves the machine.

| Speaking | Heard by |
|---|---|
| Simple Voice Chat player | Other Simple Voice Chat players, and everyone on the BVC app |
| BVC app player | Other BVC app players, and everyone on Simple Voice Chat |
| Jukebox playback | Everyone, under its own volume slider in Simple Voice Chat |

Proximity, dimensions and worlds apply on both sides. A player in the Nether does not hear a player in the Overworld.

## Setup with an embedded server

Nothing to configure. Install Simple Voice Chat and the Java mod, set `use-embedded-server` to true, and start the server.

The mod authorizes the bridge on the embedded server at startup and connects the two over the loopback interface.

## Setup with an external server

Two values are exchanged once.

1. Start the game server with Simple Voice Chat and the Java mod installed. The mod logs the block to add:

   ```
   peer "svc-bridge" {
     peerlink = "bvcpeer..."
   }
   ```

2. Add that block to the BVC server's `config.hcl` and restart the BVC server.

3. Read the BVC server's own link:

   ```
   bvc-server relay peerlink
   ```

4. Set it in the mod config and restart the game server:

   ```
   svc-bridge-peerlink: "bvcpeer..."
   ```

:::caution
The BVC server reads `peer` blocks at startup. A block added while it is running takes effect on the next restart.
:::

## Settings

| Key | Default | Description |
|---|---|---|
| `svc-bridge-peerlink` | — | The BVC server's peer link. Required in external mode. Ignored in embedded mode. |

## Players running both

A Java player can install Simple Voice Chat and the BVC app at the same time. They hear each remote speaker once. The mod asks the BVC server who holds a voice connection and leaves those players out of what it delivers to Simple Voice Chat.

:::note
On an external server this list refreshes every two seconds. A player who has just opened the BVC app may hear a speaker twice for that long.
:::

## Troubleshooting

**The log says the bridge is not peered.** The BVC server has no `peer` block naming this bridge, or `svc-bridge-peerlink` is unset. The same log line contains the block to paste.

**Simple Voice Chat players hear each other, and nobody else.** The bridge is not connected. Check the game server log for `Simple Voice Chat bridge peered`.

**Audio is dropped with a sample rate in the message.** Something is sending audio that is not 48 kHz. Simple Voice Chat plays 48 kHz only.

**Nothing in the log mentions Simple Voice Chat.** The mod did not detect it. Confirm Simple Voice Chat loads before Bedrock Voice Chat, and restart the game server.
