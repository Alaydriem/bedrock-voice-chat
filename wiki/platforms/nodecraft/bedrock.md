---
title: "Bedrock on Nodecraft"
description: "Run Bedrock Voice Chat and Bedrock Dedicated Server together on Nodecraft with the BVC launcher."
source: https://www.bedrockvoicechat.com/wiki/platforms/nodecraft/bedrock/
---
Install Bedrock Voice Chat on a Nodecraft Pro server that runs Minecraft: Bedrock Edition.

On Nodecraft, a custom launcher runs the Bedrock Voice Chat server next to your Bedrock Dedicated Server (BDS). When your Nodecraft Pro server starts, the launcher installs the Bedrock Voice Chat server and the BDS Addon, enables Beta APIs for your world, and sets up BDS.

## Before you start

You need:

- A Nodecraft Pro server. See [Nodecraft](/wiki/platforms/nodecraft/).
- A domain name, with DNS at Cloudflare or another [supported provider](/wiki/server/tls/).
- A Cloudflare API token for that zone, with DNS edit permission. See [TLS certificates](/wiki/server/tls/).

:::caution
You **MUST** use a Pro server. Lite servers stop when nobody plays. Bedrock Voice Chat then stops too.
:::

## Install Bedrock

1. Create a new **Pro** game server.
2. During server setup, select **Minecraft: Bedrock Edition**.
3. Select **Create and Deploy**. Wait for the installation to complete.

   <div class="shot"><span>Nodecraft game selection with Minecraft: Bedrock Edition</span></div>

## Upload the launcher

1. Download `bvc-launcher-linux-x64` from [GitHub Releases](https://github.com/Alaydriem/bedrock-voice-chat/releases).
2. In **Server Files** > **File Manager**, rename `bedrock_server` to `bedrock_server_vanilla`.
3. Select **Upload**, and upload `bvc-launcher-linux-x64` to the same directory. Rename it to `bedrock_server`.

   <div class="shot"><span>Nodecraft File Manager with bedrock_server and bedrock_server_vanilla</span></div>

4. Create `config/bvc.hcl` with this base configuration:

   ```hcl
   server {
       port = 28280
       quic_port = 28280

       tls {
           names = ["bvc.example.com"]

           acme {
               email = "your-cloudflare-email@example.com"
               provider = "cloudflare"
               api_token = "cfut_api-token"
           }
       }
   }
   ```

5. Replace `bvc.example.com`, the email, and the API token with your values.

:::caution
1. Use Cloudflare or another supported DNS provider. BVC then gets and renews the TLS certificate automatically.
2. Do not use your Nodecraft hostname. At this time, BVC cannot get a TLS certificate for the hostname that Nodecraft gives a Pro server.
3. Keep both ports at 28280. Do not change them.
:::

## Point your domain at the server

1. In the Pro Panel, select **Overview**. Copy the **Dedicated IP**.
2. At your DNS provider, add an `A` record for your BVC name, for example `bvc.example.com`, with the Dedicated IP as its value.
3. In Cloudflare, set the record's proxy status to **DNS only**.

BVC gets the TLS certificate the first time it starts. This can take one minute. Wait, then sign in for the first time.

:::caution
The launcher stores its certificate authority (the key that signs player certificates) in `bvc/`. Download a copy of this directory from **Server Files** and keep it. If you delete it, all client certificates that the server issued become invalid, and all players must sign in again.
:::

## Allowlist

Players in `allowlist.json` can sign in to Bedrock Voice Chat before they join the world for the first time. Other players can sign in after they join the world one time. If you remove a player from `allowlist.json`, they can still sign in to Bedrock Voice Chat.

## Connect

Your players sign in with `https://bvc.example.com:28280`. Use your domain. See [signing in](/wiki/player/signing-in/).

<div class="window"><img src="/assets/wiki/nodecraft-login.jpg" alt="BVC login window with bvc.example.com:28280 in the server address field" /></div>

## Next steps

1. Make yourself an admin from the server console:

   ```
   bvc admin grant <Gamertag>
   ```

   Use your Xbox gamertag, without quotes. To remove admin, run `bvc admin revoke <Gamertag>`.

2. Send your players to [Downloads](/wiki/start/downloads/). They can then sign in.