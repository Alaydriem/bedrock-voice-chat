---
title: Bedrock Dedicated Server Addon
description: Install and configure the BVC Addon on a Bedrock Dedicated Server.
sidebar:
  label: Bedrock Addon
  order: 6
---

The Bedrock Addon relays player positions from your Bedrock Dedicated Server to the Bedrock Voice Chat server, which is what makes proximity voice chat work.

:::caution
The Addon is **mandatory**. Without it, Bedrock Voice Chat does not know where anyone is and nothing works.
:::

For Bedrock Dedicated Server. Realms, Aternos, and other hosts that cannot enable `@minecraft/server-net` use the [no-net Addon](/wiki/server/nonet-addon/).

Install the [BVC server](/wiki/server/installation/) first. You need its URL and access token here.

## Install

1. Download the latest `.mcaddon` from [GitHub Releases](https://github.com/Alaydriem/bedrock-voice-chat/releases) or [CurseForge](https://www.curseforge.com/minecraft-bedrock/addons/bedrock-voice-chat).

2. Extract it and copy the `behavior_packs` and `resource_packs` directories into your server's matching directories.

   Never put the Addon in `worlds/`. A shared or exported world takes your BVC server URL and access token with it.

3. Create or edit `config/default/variables.json`:

   ```json
   {
       "bvc_access_token": "<MUST MATCH server.minecraft.access_token>",
       "bvc_server": "https://example.bedrockvc.stream",
       "bvc_minimum_players": 1,
       "bvc_world_name": "My SMP"
   }
   ```

   | Key | Description |
   |---|---|
   | `bvc_access_token` | Must match `server.minecraft.access_token` in `config.hcl`. |
   | `bvc_server` | Public URL of your BVC server. No trailing slash. Include the port if it is not 443. |
   | `bvc_minimum_players` | Players who must be online before positions are relayed. Defaults to 1. |
   | `bvc_world_name` | Display name for this world in the app's [chat](/wiki/player/chat/) picker. Defaults to "Minecraft world". |

   Set `bvc_world_name`. BDS scripting cannot read your level name. This is the only label the app has.

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

5. Enable **Beta APIs** for your world.

6. Add the packs to your world's behavior and resource pack lists, using the UUID for the variant you installed:

   | Variant | Behavior pack UUID |
   |---|---|
   | Full | `6fb24263-357a-407c-aebe-681f50b2de50` |
   | No-net | `508a43ed-b95e-40cf-98eb-d78f2d72caa4` |

   Take the version from the `manifest.json` inside the pack you downloaded. It changes every release.

7. Start or restart BDS.

## Verify

Set `content-log-console-output-enabled=true` in `server.properties`. On world load you should see:

```
[BVC] Connecting to: <your server>
```

No line means the Addon is not running, or `bvc_server` is unset.

Then check the server is receiving positions:

```bash
curl -H "Authorization: Bearer <your token>" https://example.bedrockvc.stream/api/position
```

An empty response means BDS is not relaying. A populated one means the whole chain works.

## What the Addon adds

Beyond position relay, it provides the [in-game commands](/wiki/player/in-game-commands/), the [jukebox](/wiki/player/using-the-jukebox/) block, and the [chat](/wiki/player/chat/) relay that lets players read and send in-game chat from the app.

## Troubleshooting

**No `[BVC] Connecting to:` line.** Beta APIs are off, the pack is not in the world's list, or `permissions.json` is missing a module.

**Connecting, but nobody hears anyone.** Whitelist your players — BVC is deny-by-default. See [players and permissions](/wiki/server/players-and-permissions/).

**Nothing relays when you test alone.** `bvc_minimum_players` defaults to 1. If you raised it, drop it back to 1 to test.

**`/api/config` does not respond.** The problem is the BVC server, not the Addon. Back to [installation](/wiki/server/installation/).
