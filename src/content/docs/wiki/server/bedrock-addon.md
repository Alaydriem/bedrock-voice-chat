---
title: Bedrock Dedicated Server Addon
description: Install and configure the BVC Addon on a Bedrock Dedicated Server.
sidebar:
  label: Bedrock Addon
  order: 6
---

The Bedrock Addon sends player positions from your Bedrock Dedicated Server (BDS) to the Bedrock Voice Chat server. Proximity voice chat needs these positions.

:::caution
The Addon is **mandatory**. Without it, Bedrock Voice Chat does not receive player positions, and players cannot hear each other.
:::

This page is for Bedrock Dedicated Server. For Realms, Aternos, and other hosts that do not allow `@minecraft/server-net`, use the [no-net Addon](/wiki/server/nonet-addon/).

Install the [BVC server](/wiki/server/installation/) first. You need its URL and access token for these steps.

## Install

1. Download the latest `.mcaddon` from [GitHub Releases](https://github.com/Alaydriem/bedrock-voice-chat/releases) or [CurseForge](https://www.curseforge.com/minecraft-bedrock/addons/bedrock-voice-chat).

2. Extract it. Copy the `behavior_packs` and `resource_packs` directories into the directories with the same names on your server.

   Do not put the Addon in `worlds/`. If you do, a shared or exported world contains your BVC server URL and access token.

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
   | `bvc_minimum_players` | Number of players who must be online before the Addon sends positions. The default is 1. |
   | `bvc_world_name` | Name of this world in the app's [chat](/wiki/player/chat/) world list. The default is "Minecraft world". |

   Set `bvc_world_name`. BDS scripts cannot read the level name. The app shows only the `bvc_world_name` value.

4. Allow the modules that the Addon uses in `config/default/permissions.json`:

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

5. Enable [Beta APIs](/wiki/server/beta-apis/) for your world.

6. Add the packs to your world's behavior pack and resource pack lists. Use the UUID of the variant that you installed. On BDS, this is the Full variant.

   | Variant | Behavior pack UUID |
   |---|---|
   | Full | `6fb24263-357a-407c-aebe-681f50b2de50` |
   | No-net | `508a43ed-b95e-40cf-98eb-d78f2d72caa4` |

   Copy the version from the `manifest.json` file in the pack that you downloaded. The version changes with each release.

7. Start or restart BDS.

## Verify

Set `content-log-console-output-enabled=true` in `server.properties`. When the world loads, the console shows:

```
[BVC] Connecting to: <your server>
```

If the line does not show, the Addon is not running, or `bvc_server` is not set.

Then check that the server receives positions:

```bash
curl -H "Authorization: Bearer <your token>" https://example.bedrockvc.stream/api/position
```

If the response is empty, BDS does not send positions. If the response lists players, BDS sends positions and the BVC server receives them.

## Other Addon features

The Addon also adds:

- [In-game commands](/wiki/player/in-game-commands/).
- The [jukebox](/wiki/player/using-the-jukebox/) block.
- [Chat](/wiki/player/chat/) sync. Players can read and send in-game chat from the app.

## Troubleshooting

**No `[BVC] Connecting to:` line.** Beta APIs are off, the pack is not in the world's pack list, or a module is missing from `permissions.json`.

**The Addon connects, but nobody hears anyone.** Let your players in. A player can sign in after they join your game server once, or after you add them. See [players and permissions](/wiki/server/players-and-permissions/).

**No positions when you test alone.** `bvc_minimum_players` has a default of 1. If you set a higher value, set it to 1 for the test.

**The BVC server does not respond to `curl`.** The problem is in the BVC server. Go to [installation](/wiki/server/installation/).
