---
title: PowerNukkitX on Nodecraft
description: Run Bedrock Voice Chat on a Nodecraft PowerNukkitX server with the embedded BVC server.
sidebar:
  label: PowerNukkitX
  order: 4
---

Install Bedrock Voice Chat on a Nodecraft Pro server that runs PowerNukkitX. The BVC server runs inside the Minecraft server (embedded mode). For the full list of plugin settings, see [PowerNukkitX](/wiki/server/powernukkitx/).

## Before you start

You need:

- A Nodecraft Pro server. See [Nodecraft](/wiki/platforms/nodecraft/).
- One of these:
  - Nothing more. BVC gives the server a [play name](/wiki/server/tls/#play-name).
  - A domain name, with DNS at Cloudflare or another [supported provider](/wiki/server/tls/), and a Cloudflare API token for that zone with DNS edit permission. See [TLS certificates](/wiki/server/tls/).

:::caution
You **MUST** use a Pro server. Lite servers stop when nobody plays. Bedrock Voice Chat then stops too.
:::

## Install PowerNukkitX

<!-- TODO: confirm the Nodecraft panel steps and labels for a PowerNukkitX server. -->

1. Create a new **Pro** game server.
2. During server setup, select **PowerNukkitX**.
3. Select **Create and Deploy**. Wait for the installation to complete.
4. Add `--add-opens java.base/java.lang=ALL-UNNAMED` to the server's Java startup options.

   <div class="shot"><span>Nodecraft server setup with PowerNukkitX selected</span></div>

:::caution
Without `--add-opens java.base/java.lang=ALL-UNNAMED`, the plugin does not enable.
:::

## Upload the plugin

1. Download `bedrock-voice-chat-powernukkitx-<version>-bundled.jar` from [GitHub Releases](https://github.com/Alaydriem/bedrock-voice-chat/releases).
2. In **Server Files** > **File Manager**, open the `plugins` directory.
3. Select **Upload**, and upload the `.jar` file that you downloaded.

   <div class="shot"><span>Nodecraft File Manager showing the plugins directory with the Bedrock Voice Chat jar</span></div>

4. In the Pro Panel, select **Overview**. Copy the **Dedicated IP**.
5. Create `plugins/BedrockVoiceChat/config.yml`.

   Without a domain:

   ```yaml
   minimum-players: 1
   use-embedded-server: true
   embedded-config:
     server:
       port: 28280
       quic_port: 28280
       tls:
         play:
           address: "203.0.113.10"
   ```

   With a domain:

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

6. Replace `203.0.113.10` with your Dedicated IP. With a domain, replace `bvc.example.com`, the email, and the API token with your values.

:::caution
1. Keep both ports at 28280. Do not change them.
2. Do not use your Nodecraft hostname. At this time, BVC cannot get a TLS certificate for the hostname that Nodecraft gives a Pro server.
3. With a domain, use Cloudflare or another supported DNS provider. BVC then gets and renews the TLS certificate automatically.
:::

## Point your domain at the server

Skip this section if you use a play name.

1. At your DNS provider, add an `A` record for your BVC name, for example `bvc.example.com`, with the Dedicated IP as its value.
2. In Cloudflare, set the record's proxy status to **DNS only**.

BVC gets the TLS certificate the first time it starts. This can take one minute. Wait, then sign in for the first time. If players cannot sign in after BVC gets its first certificate, restart the server.

:::caution
The plugin stores its certificate authority (the key that signs player certificates) in `plugins/BedrockVoiceChat/`. Download a copy of this directory from **Server Files** and keep it. If you delete it, all client certificates that the server issued become invalid, and all players must sign in again.
:::

## Chat sync

The plugin sends in-game chat to the BVC app, and messages from the app to the game. No server setting is needed. See [chat](/wiki/player/chat/).

## Connect

Your players sign in with `https://bvc.example.com:28280`. Use your play name or your domain. To find the play name, look in the server console for `Play name assigned`. See [signing in](/wiki/player/signing-in/).

<div class="window"><img src="/assets/wiki/nodecraft-login.jpg" alt="BVC login window with bvc.example.com:28280 in the server address field" /></div>

## Next steps

1. Make yourself an admin from the server console:

   ```
   bvc admin grant <Gamertag>
   ```

   Use your Xbox gamertag. See [Java commands](/wiki/player/in-game-commands/java/#admin).

2. Send your players to [Downloads](/wiki/start/downloads/). They can then sign in.
