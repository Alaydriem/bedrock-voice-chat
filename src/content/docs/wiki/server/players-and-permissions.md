---
title: Players and permissions
description: Whitelisting, the three permissions, and bootstrapping your first admin.
sidebar:
  label: Players and permissions
  order: 9
---

Bedrock Voice Chat is deny-by-default. Players appear automatically once your game server relays them.

:::caution
No player can sign in until you whitelist them. A player who has never been added is refused with `AUTH02`.
:::

Everything here uses the server executable, which doubles as a CLI. Full reference: [CLI](/wiki/reference/cli/).

## Bootstrapping the first admin

A fresh deployment has no admin. The commands that create one talk to the database directly, not over the API. Run them on the server host, where `config.hcl` is.

```bash
# On the server host
bvc admin generate-code -p Alice -g minecraft -d 3600
# -> Code: ABCD1234

# Anywhere with network access
bvc login --code ABCD1234

# Back on the server host
bvc admin bootstrap -p Alice -g minecraft
```

`admin generate-code` creates the player record if it does not exist, which is what makes it usable before anyone exists. `admin bootstrap` grants admin to a player who already exists.

Confirm:

```bash
bvc whoami
```

It prints the active identity, server URL, certificate expiry, and effective permissions. `admin` should be in the list.

### On an embedded Java server

A Fabric or PaperMC server running the [embedded server](/wiki/server/java-mod/#integrated-or-external-bvc-server) has no `bvc` executable. Run this from the game server console instead:

```
/bvc admin grant <Gamertag>
```

Use the Xbox gamertag the player signs in with. The command creates the player record if it does not exist, so the player can sign in immediately. See [Java commands](/wiki/player/in-game-commands/java/#admin).

## Adding players

Once you have an admin identity, the rest goes over HTTPS with mTLS and can be run from anywhere.

```bash
bvc user add -p Bob -g minecraft
bvc user generate-code -p Bob -g minecraft
```

Give Bob the code. He redeems it with `bvc login`, or just signs in through the app.

`user add` returns 409 if the player already exists. `user generate-code` returns 404 if they do not. Run `user add` first.

Codes default to one hour and cap at 24.

An admin can also add players from **Settings → Manage Players** in the app.

<div class="window"><img src="/assets/wiki/settings-manage-players.jpg" alt="Manage Players page listing the players who can sign in to the server" /></div>

## Removing players

```bash
bvc user banish -p Bob -g minecraft
bvc user banish -p Bob -g minecraft --banish false
```

Banishing revokes the player's certificate and closes their voice session. They are removed while connected.

Banishing is reversible and keeps the record. There is no hard delete from the CLI.

Unbanishing does not restore the revoked certificate. The player signs in and is issued a new one.

## Permissions

| Permission | Grants |
|---|---|
| `audio_upload` | Upload clips to the [audio library](/wiki/player/audio-library/). |
| `audio_delete` | Delete clips. |
| `admin` | User and permission management. |

Peering is not a player permission. A peer is declared in [`server.peers`](/wiki/reference/configuration/#serverpeers). See [peering](/wiki/reference/peering/).

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

`allow` and `deny` are explicit overrides. `clear` removes the override, and the config default applies again. `clear` is not `deny`.

Empty output from `list` means the player has no overrides and is governed entirely by the defaults.

**Manage Players** sets the same three overrides from the settings icon on a player's row. **Default** clears the override.

On an embedded Java server, the console sets the `admin` override with `/bvc admin grant`, `revoke`, and `deny`. `revoke` is the console form of `clear`. See [Java commands](/wiki/player/in-game-commands/java/#admin).

<div class="window"><img src="/assets/wiki/settings-manage-permissions.jpg" alt="Permissions dialog for a player with Default, Allow, and Deny for Administrator, Upload sounds, and Delete sounds" /></div>

## A reasonable starting policy

On a public server, default `audio_upload` to false and grant it to people you trust. Storage is your disk and there is no per-player quota.

On a friends-and-family server the defaults are fine as they are.
