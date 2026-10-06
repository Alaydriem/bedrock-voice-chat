---
title: Bedrock on Nodecraft
description: Run Bedrock Voice Chat and Bedrock Dedicated Server together on Nodecraft.
# Draft until the Nodecraft BVC + BDS launcher ships. Drafts are left out of
# production builds and the sidebar. When this goes live, remove `draft` and add
# a Bedrock row to the table in platforms/nodecraft/index.md.
draft: true
sidebar:
  label: Bedrock
  order: 5
---

This guide runs Bedrock Dedicated Server and the BVC server together on one Nodecraft server, with the Nodecraft BVC launcher. For the full list of Addon settings, see [Bedrock Addon](/wiki/server/bedrock-addon/).

## Before you start

You need:

- A Nodecraft server. See [Nodecraft](/wiki/platforms/nodecraft/).
- A domain name, with DNS at Cloudflare or another [supported provider](/wiki/server/tls/).
- A Cloudflare API token for that zone, with DNS edit permission. See [TLS certificates](/wiki/server/tls/).

## Install the launcher

<!-- TODO: launcher selection steps, once Nodecraft publishes the launcher. -->

<div class="shot"><span>Nodecraft panel, BVC + BDS launcher selected</span></div>

## Point your domain at the server

1. In the Pro Panel, select **Overview**. Copy the **Numeric IP**.

   <div class="shot"><span>Nodecraft Overview, Numeric IP</span></div>

2. At your DNS provider, add an `A` record for your BVC name, for example `bvc.example.com`, with the Numeric IP as its value.
3. In Cloudflare, set the record's proxy status to **DNS only**. A proxied record does not carry voice traffic.

## Configure the BVC server

<!-- TODO: where the launcher keeps config.hcl, which ports it binds, and how
     server.minecraft.access_token is set. Verify against the launcher. -->

<div class="shot"><span>Nodecraft Server Files, BVC config.hcl</span></div>

## Install the Addon

1. Download the latest `.mcaddon` from [GitHub Releases](https://github.com/Alaydriem/bedrock-voice-chat/releases) or [CurseForge](https://www.curseforge.com/minecraft-bedrock/addons/bedrock-voice-chat).
2. Extract it. In **Server Files**, upload the `behavior_packs` and `resource_packs` directories into the server's matching directories.

   Never put the Addon in `worlds/`. A shared or exported world takes your BVC server URL and access token with it.

   <div class="shot"><span>Nodecraft Server Files, behavior_packs with the BVC Addon</span></div>

3. Create or edit `config/default/variables.json`:

   ```json
   {
       "bvc_access_token": "<MUST MATCH server.minecraft.access_token>",
       "bvc_server": "https://bvc.example.com:8444",
       "bvc_minimum_players": 1,
       "bvc_world_name": "My SMP"
   }
   ```

4. Grant the modules the Addon uses in `config/default/permissions.json`:

   ```json
   {
       "allowed_modules": [
           "@minecraft/server",
           "@minecraft/server-admin",
           "@minecraft/server-net",
           "@minecraft/server-ui"
       ]
   }
   ```

5. Enable **Beta APIs** for your world. See [Beta APIs](/wiki/server/beta-apis/).

The [Bedrock Addon](/wiki/server/bedrock-addon/) page describes each key.

## Connect

Your players sign in with `https://bvc.example.com:8444`. Use your domain. See [signing in](/wiki/player/signing-in/).

<!-- TODO: confirm the public sign-in port the launcher uses. -->

## Next steps

1. [Add your players to the whitelist](/wiki/server/players-and-permissions/). BVC denies all players that are not on it.
2. Send your players to [Downloads](/wiki/start/downloads/).
