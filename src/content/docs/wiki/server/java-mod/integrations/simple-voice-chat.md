---
title: Simple Voice Chat
description: Bridge Simple Voice Chat and Bedrock Voice Chat on one Minecraft server.
sidebar:
  label: Simple Voice Chat
  order: 3
---

Bedrock Voice Chat bridges [Simple Voice Chat](https://modrinth.com/plugin/simple-voice-chat), letting Java players on Simple Voice Chat and players on the BVC app hear each other in the same world.

## Requirements

| Component | Where |
|---|---|
| Bedrock Voice Chat Java mod | Fabric or PaperMC. See [Java mod](/wiki/server/java-mod/) |
| Simple Voice Chat | [The server-side plugin or mod](https://modrinth.com/plugin/simple-voice-chat) |
| A BVC server | Embedded in the game server, or external |

## Setup with an embedded server

Bedrock Voice Chat will automatically configure Simple Voice Chat when the embedded server is enabled. Simply:

1. Install both BVC and SVC plugin / mod to your PaperMC or Fabric Server
2. Enable BVC's embedded server [see configuration per your platform for details](/wiki/server/java-mod/)

## Setup with an external server

If you're using an external Bedrock Voice Chat server, you'll need to run one setup step on your BVC server

1. Install both BVC and SVC plugin / mod to your PaperMC or Fabric Server
2. Configure BVC in your Java mod to use an external server
3. Start the game server. You'll see output that looks similar to the following

   ```
   peers "svc-bridge" {
     peerlink = "bvcpeer..."
   }
   ```

4. Add that block to the BVC server's `config.hcl` under the servers section, and restart the BVC server.

   ```hcl
   server {
      tls {}
      bedrock {}

      peers "svc-bridge" {
         peerlink = "bvcpeer..."
      }
   }
   ```
5. When BVC starts, you'll see a log message that looks as follows. This is Bedrock Voice Chat's peer link code.

   ```
   INFO bvc_server_lib::runtime: this server's peer link peerlink=bvcpeer....
   ```
   :::tip[Manual Peerlink]
      You can also fetch the peer link code via the CLI

      ```
      bvc-server relay peerlink
      ```
   :::

6. Set it in the mod config and restart the game server:

   ```
   svc-bridge-peerlink: "bvcpeer...the-link-you-saw-in-bvc-server-startup-logs"
   ```

## Settings

| Key | Default | Description |
|---|---|---|
| `svc-bridge-peerlink` | — | The BVC server's peer link. Required in external mode. Ignored in embedded mode. |
