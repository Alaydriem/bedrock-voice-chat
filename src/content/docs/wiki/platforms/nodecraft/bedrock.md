---
title: Bedrock on Nodecraft
description: Run Bedrock Voice Chat and Bedrock Dedicated Server together on Nodecraft with the BVC launcher.
# Draft until the BVC launcher is in a release. Drafts are left out of
# production builds and the sidebar. When this goes live, remove `draft` and add
# a Bedrock row to the table in platforms/nodecraft/index.md.
draft: true
sidebar:
  label: Bedrock
  order: 5
---

This guide details how to install Bedrock Voice Chat on Bedrock on a Nodecraft Pro server.

With Nodecraft, the Bedrock Voice Chat server runs alongside your BDS instance through a custom launcher. When your Nodecraft Pro server starts, the launcher installs the Bedrock Voice Chat server and the BDS Addon, enables Beta APIs for your world, and sets up your BDS instance. It could not be simpler.

## Before you start

You need:

- A Nodecraft Pro server. See [Nodecraft](/wiki/platforms/nodecraft/).
- A domain name, with DNS at Cloudflare or another [supported provider](/wiki/server/tls/).
- A Cloudflare API token for that zone, with DNS edit permission. See [TLS certificates](/wiki/server/tls/).

:::tip
You **must** use a Pro server. Lite servers suspend themselves after a period of inactivity, which causes problems with Bedrock Voice Chat.
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

:::tip
1. Use Cloudflare or another ACME provider. BVC then issues and renews the TLS certificate automatically.
2. Do not use your Nodecraft hostname. At this time, BVC cannot get a TLS certificate for the hostname that Nodecraft gives a Pro server.
3. Leave both ports at 28280. Do not change them.
:::

## Point your domain at the server

1. In the Pro Panel, select **Overview**. Copy the **Dedicated IP**.
2. At your DNS provider, add an `A` record for your BVC name, for example `bvc.example.com`, with the Dedicated IP as its value.
3. In Cloudflare, set the record's proxy status to **DNS only**.

The first start issues the TLS certificate. Bedrock Voice Chat can take a minute to start fully while it gets the certificate. Wait before you sign in for the first time.

:::caution
The launcher keeps its mTLS CA in `bvc/`. Download a copy of this directory from **Server Files** and keep it. If you delete it, all client certificates that the server issued become invalid, and all players must sign in again.
:::

## Allowlist

Players in `allowlist.json` can sign in before they join the world for the first time. Removing a player from the allowlist does not remove them from Bedrock Voice Chat.

## Connect

Your players sign in with `https://bvc.example.com:28280`. Use your domain. See [signing in](/wiki/player/signing-in/).

<div class="window"><img src="/assets/wiki/nodecraft-login.jpg" alt="BVC login window with bvc.example.com:28280 in the server address field" /></div>

## Next steps

1. Make yourself an admin from the server console:

   ```
   bvc admin grant <Gamertag>
   ```

   Use your Xbox gamertag, without quotes. `bvc admin revoke <Gamertag>` removes it.

2. Send your players to [Downloads](/wiki/start/downloads/). They can then sign in.
