---
title: "Simple Voice Chat"
description: "Bridge Simple Voice Chat and Bedrock Voice Chat on one Minecraft server."
source: https://www.bedrockvoicechat.com/wiki/server/java-mod/integrations/simple-voice-chat/
---
Bedrock Voice Chat connects to [Simple Voice Chat](https://modrinth.com/plugin/simple-voice-chat) (SVC). Java players on Simple Voice Chat and players on the BVC app can then hear each other in the same world.

## Requirements

| Component | Where |
|---|---|
| Bedrock Voice Chat Java mod | Fabric or PaperMC. See [Java mod](/wiki/server/java-mod/). |
| Simple Voice Chat | [The server plugin or mod](https://modrinth.com/plugin/simple-voice-chat). |
| A BVC server | Embedded in the game server, or external. |

## Setup with an embedded server

When you use the embedded server, BVC configures the bridge automatically.

1. Install the BVC mod and Simple Voice Chat on your PaperMC or Fabric server.
2. Enable the embedded server. See [Fabric](/wiki/server/java-mod/fabric/#embedded-mode) or [PaperMC](/wiki/server/java-mod/papermc/#embedded-mode).

## Setup with an external server

With an external BVC server, connect the game server and the BVC server with peer links. A peer link is a code that lets two servers trust each other. Add the game server's peer link to the BVC server, and the BVC server's peer link to the game server.

1. Install the BVC mod and Simple Voice Chat on your PaperMC or Fabric server.
2. Configure the BVC mod to use an [external server](/wiki/server/java-mod/#external-or-embedded-server).
3. Start the game server. The console shows a `peers` block with the game server's peer link:

   ```
   peers "svc-bridge" {
     peerlink = "bvcpeer..."
   }
   ```

4. Add this block inside the `server` block of the BVC server's `config.hcl`. Restart the BVC server.

   ```hcl
   server {
      tls {}
      bedrock {}

      peers "svc-bridge" {
         peerlink = "bvcpeer..."
      }
   }
   ```

5. When the BVC server starts, it logs its own peer link:

   ```
   INFO bvc_server_lib::runtime: this server's peer link peerlink=bvcpeer....
   ```

   :::tip[Get the peer link]
   To show the peer link at any time, run:

   ```
   bvc-server relay peerlink
   ```
   :::

6. Set `svc-bridge-peerlink` to the BVC server's peer link in the mod configuration. This is `config/bedrock-voice-chat.json` on Fabric, or `plugins/BedrockVoiceChat/config.yml` on PaperMC. Restart the game server.

   ```
   svc-bridge-peerlink: "bvcpeer...the-link-from-the-bvc-server-log"
   ```

## Settings

| Key | Default | Description |
|---|---|---|
| `svc-bridge-peerlink` | None | The BVC server's peer link. Required in external mode. Ignored in embedded mode. |