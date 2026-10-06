---
title: PaperMC on Nodecraft
description: Run Bedrock Voice Chat on a Nodecraft PaperMC server with the embedded BVC server.
sidebar:
  label: PaperMC
  order: 3
---

This guide installs the Bedrock Voice Chat PaperMC plugin on a Nodecraft server, in embedded mode. The BVC server runs inside the Minecraft server. For the full list of plugin settings, see [PaperMC](/wiki/server/java-mod/papermc/).

## Before you start

You need:

- A Nodecraft Pro server. See [Nodecraft](/wiki/platforms/nodecraft/).
- A domain name, with DNS at Cloudflare or another [supported provider](/wiki/server/tls/).
- A Cloudflare API token for that zone, with DNS edit permission. See [TLS certificates](/wiki/server/tls/).

:::tip
You **must** use a Pro server. Lite servers suspend themselves after a period of inactivity, which causes problems with Bedrock Voice Chat.
:::

## Install Paper

1. Create a new **Pro** game server.
2. During server setup, add a mod for **Paper**.
3. Select **Create and Deploy**. Wait for the installation to complete.

   <div class="window"><img src="/assets/wiki/nodecraft-install-paper.jpg" alt="Nodecraft Browse Mod to Install dialog filtered to Paper" /></div>

## Upload the plugin

1. Download [Bedrock Voice Chat](https://modrinth.com/mod/bedrock-voice-chat) from Modrinth. Select the Paper version that matches your Minecraft version. For crossplay, also download Simple Voice Chat, Geyser, and Floodgate.
2. In **Server Files** > **File Manager**, open the `plugins` directory.
3. Select **Upload**, and upload all four `.jar` files.

   <div class="window"><img src="/assets/wiki/nodecraft-paper-plugins.jpg" alt="Nodecraft File Manager showing the plugins directory with the Bedrock Voice Chat, Geyser, Floodgate, and Simple Voice Chat jars" /></div>

4. Create `plugins/BedrockVoiceChat/config.yml` with this base configuration:

   ```yaml
   minimum-players: 1
   use-embedded-server: true
   embedded-config:
     server:
       port: 28280
       quic_port: 28280
       tls:
         names:
           - "bvc.example.com"
         ips: []
         acme:
           email: "your-cloudflare-email@example.com"
           provider: "cloudflare"
           api_token: "cfut_api-token"
   ```

5. Replace `bvc.example.com`, the email, and the API token with your values.

:::tip
1. Use Cloudflare or another ACME provider. BVC then issues and renews the TLS certificate automatically.
2. Do not use your Nodecraft hostname. At this time, BVC cannot get a TLS certificate for the hostname that Nodecraft gives a Pro server.
3. Leave both ports at 28280. Do not change them.
:::

## Point your domain at the server

1. In the Pro Panel, select **Overview**. Copy the **Dedicated IP**.
2. At your DNS provider, add an `A` record for your BVC name, for example `bvc.example.com`, with the Dedicated IP as its value.
3. In Cloudflare, set the record's proxy status to **DNS only**.

The first start issues the TLS certificate. Bedrock Voice Chat can take a minute to start fully while it gets the certificate. Wait before you sign in for the first time. You may need to restart the Java server _after_ the first certificate is issued.

:::caution
The plugin keeps its mTLS CA in `plugins/BedrockVoiceChat/`. Download a copy of this directory from **Server Files** and keep it. If you delete it, all client certificates that the server issued become invalid, and all players must sign in again.
:::

## Chat sync

Chat sync needs this line in `server.properties`:

```properties
enforce-secure-profile=false
```

Edit `server.properties` in **Server Files**, then restart the server. See [chat](/wiki/player/chat/).

## Connect

Your players sign in with `https://bvc.example.com:28280`. Use your domain. See [signing in](/wiki/player/signing-in/).

<div class="window"><img src="/assets/wiki/nodecraft-login.jpg" alt="BVC login window with bvc.example.com:28280 in the server address field" /></div>

## Next steps

1. Make yourself an admin from the server console:

   ```
   /bvc admin grant <Gamertag>
   ```

   Use your Xbox gamertag. Put it in quotes if it contains a space. See [Java commands](/wiki/player/in-game-commands/java/#admin).

2. Send your players to [Downloads](/wiki/start/downloads/). They can then sign in.
