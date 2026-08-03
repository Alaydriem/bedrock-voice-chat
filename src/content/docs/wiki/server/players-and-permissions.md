---
title: Players and permissions
description: Whitelisting, the four permissions, and bootstrapping your first admin.
sidebar:
  label: Players and permissions
  order: 8
---

BVC is deny-by-default. Players show up automatically once your game server relays them, but none of them can sign in until you add them.

Everything here uses the server executable, which doubles as a CLI. Full reference: [CLI](/wiki/reference/cli/).

## Bootstrapping the first admin

A fresh deployment has no admin, and the commands that create one talk to the database directly rather than over the API. Run these on the server host, where `config.hcl` is.

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

## Adding players

Once you have an admin identity, the rest goes over HTTPS with mTLS and can be run from anywhere.

```bash
bvc user add -p Bob -g minecraft
bvc user generate-code -p Bob -g minecraft
```

Give Bob the code. He redeems it with `bvc login`, or just signs in through the app.

`user add` returns 409 if the player already exists. `user generate-code` returns 404 if they do not — use `user add` first.

Codes default to one hour and cap at 24.

## Removing players

```bash
bvc user banish -p Bob -g minecraft
bvc user banish -p Bob -g minecraft --banish false
```

Banishing is reversible and keeps the record. There is no hard delete from the CLI.

## Permissions

| Permission | Grants |
|---|---|
| `audio_upload` | Upload clips to the [audio library](/wiki/player/audio-library/). |
| `audio_delete` | Delete clips. |
| `admin` | User and permission management. |
| `peer_link` | Establish cross-server peer links. Early access. |

### Server-wide defaults

```hcl
permissions {
    defaults = {
        audio_upload = true
        audio_delete = false
        admin        = false
        peer_link    = false
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

`allow` and `deny` are explicit overrides. `clear` removes the override so the config default applies again — which is not the same as `deny`.

Empty output from `list` means the player has no overrides and is governed entirely by the defaults.

## A reasonable starting policy

On a public server, default `audio_upload` to false and grant it to people you trust. Storage is your disk and there is no per-player quota.

On a friends-and-family server the defaults are fine as they are.
