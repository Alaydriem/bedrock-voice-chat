---
title: "Players and permissions"
description: "Whitelisting, the three permissions, and bootstrapping your first admin."
source: https://www.bedrockvoicechat.com/wiki/server/players-and-permissions/
---
Bedrock Voice Chat lets a player sign in only when the server knows that player. The server knows a player in one of two ways:

1. The player joins your game server. The Bedrock Addon or the Java mod sends the player to BVC.
2. An admin adds the player in the app or with the CLI.

:::caution
A player who has not joined your game server and has not been added cannot sign in. The app shows `AUTH02`.
:::

The `bvc` commands below are the server executable, run as a command-line tool. See the [CLI reference](/wiki/reference/cli/).

## Bootstrapping the first admin

A new deployment has no admin. The commands that create the first admin write to the database directly. They do not use the HTTP API. Run them on the server host, in the directory that contains `config.hcl`.

```bash
# On the server host
bvc admin generate-code -p Alice -g minecraft -d 3600
# -> Code: ABCD1234

# Anywhere with network access
bvc login --code ABCD1234

# Back on the server host
bvc admin bootstrap -p Alice -g minecraft
```

`admin generate-code` and `admin bootstrap` both create the player record if it does not exist. You can use them on an empty database. `admin bootstrap` gives the player admin.

To check the result, run:

```bash
bvc whoami
```

The output shows the active identity, the server URL, the certificate expiry, and the effective permissions. `admin` must be in the list.

### On an embedded Java server

A Fabric or PaperMC server that runs the [embedded server](/wiki/server/java-mod/#external-or-embedded-server) has no `bvc` executable. Run this command from the game server console:

```
/bvc admin grant <Gamertag>
```

Use the Xbox gamertag that the player signs in with. The command creates the player record if it does not exist. The player can then sign in. See [Java commands](/wiki/player/in-game-commands/java/#admin).

## Adding players

After you have an admin identity, you can run the remaining commands from any computer. They connect to the server's HTTPS API with your client certificate (mTLS).

```bash
bvc user add -p Bob -g minecraft
bvc user generate-code -p Bob -g minecraft
```

Give Bob the code. Bob uses the code with `bvc login`, or signs in through the app.

`user add` returns 409 if the player already exists. `user generate-code` returns 404 if the player does not exist. Run `user add` first.

A code expires after one hour by default. The maximum is 24 hours.

An admin can also add players from **Settings → Manage Players** in the app.

<div class="window"><img src="/assets/wiki/settings-manage-players.jpg" alt="Manage Players page listing the players who can sign in to the server" /></div>

## Removing players

```bash
bvc user banish -p Bob -g minecraft
bvc user banish -p Bob -g minecraft --banish false
```

Banishing revokes the player's certificate and closes their voice session. If the player is connected, BVC disconnects them immediately.

You can reverse a banish. The player record stays. The CLI cannot delete a player record.

Unbanishing does not restore the revoked certificate. The player signs in again and gets a new certificate.

## Permissions

| Permission | Grants |
|---|---|
| `audio_upload` | Upload clips to the [audio library](/wiki/player/audio-library/). |
| `audio_delete` | Delete clips. |
| `admin` | Manage players and permissions. |

Peering is not a player permission. You declare a peer in [`server.peers`](/wiki/reference/configuration/#serverpeers). See [peering](/wiki/reference/peering/).

### Server-wide defaults

```hcl
permissions {
    defaults = {
        audio_upload = true
        audio_delete = false
        admin        = false
    }
}
```

### Per-player overrides

```bash
bvc permission allow -p Bob -g minecraft --permission audio_upload
bvc permission deny  -p Bob -g minecraft --permission audio_upload
bvc permission clear -p Bob -g minecraft --permission audio_upload
bvc permission list  -p Bob -g minecraft
```

`allow` and `deny` set an override for one player. `clear` removes the override. The default from `config.hcl` then applies again.

If `list` prints nothing, the player has no overrides. Only the defaults apply.

In the app, **Manage Players** sets the same three overrides. Click the settings icon on the player's row. **Default** removes the override.

On an embedded Java server, use `/bvc admin grant`, `revoke` and `deny` in the console to set the `admin` override. `revoke` does the same as `clear`. See [Java commands](/wiki/player/in-game-commands/java/#admin).

<div class="window"><img src="/assets/wiki/settings-manage-permissions.jpg" alt="Permissions dialog for a player with Default, Allow, and Deny for Administrator, Upload sounds, and Delete sounds" /></div>

## Recommended defaults

On a public server, set `audio_upload` to `false` by default. Give it to players you trust. Uploaded clips use disk space on your server, and there is no limit for each player.

On a private server, keep the defaults.