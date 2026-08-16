---
title: CLI
description: The server executable's command line — identities, users, permissions, bootstrapping.
sidebar:
  label: CLI
  order: 4
---

The Bedrock Voice Chat server executable is also a CLI, letting you manage identities, users, and permissions either locally or against a remote server.

Available since beta.11.

## Global flags

| Flag | Env | Default | Notes |
|---|---|---|---|
| `-c, --config-file <path>` | — | `config.hcl` | Used by `server` and `admin *` only. |
| `--server-url <url>` | `BVC_SERVER` | `https://127.0.0.1:3000` | Target for `login`. Other commands read it from the stored identity. |
| `--identity <gamertag:game>` | `BVC_IDENTITY` | — | Which stored identity to act as, when more than one exists. |

## Identity

| Command | Auth | Transport | Does |
|---|---|---|---|
| `login` | none | `POST /api/auth/code` | Redeem a one-time code for an mTLS certificate bundle and store it. |
| `logout` | mTLS | local | Delete the active stored identity. |
| `whoami` | mTLS | `GET /api/auth/introspect` | Print identity, server URL, certificate expiry, and effective permissions. |

`login` takes one flag: `--code <code>`, prompted on stdin if omitted.

The code is the whole credential. It identifies the player it was issued for, and the server returns the gamertag and the game.

## Server

| Command | Does |
|---|---|
| `server` | Start the BVC server, HTTP and QUIC. Reads `--config-file`. |

## Bootstrap

These talk to the database directly. Run them on the server host with `config.hcl` available. A fresh deployment has no admin, and there is nobody to authenticate as.

| Command | Flags | Does |
|---|---|---|
| `admin bootstrap` | `-p, --player <name>`; `-g, --game <minecraft>` | Grant `admin` to an existing player. |
| `admin generate-code` | `-p, --player <name>`; `-g, --game <minecraft>`; `-d, --duration <secs>` | Generate a one-time login code, creating the player record if needed. |

`-d` defaults to 3600 and caps at 86400.

`admin generate-code` creates the player if missing. `admin bootstrap` requires the player to exist.

## Users

Over HTTPS with mTLS. Requires an active admin identity. `whoami` should list `admin`.

| Command | Endpoint | Does |
|---|---|---|
| `user add` | `POST /api/admin/user` | Create a player with certificate and keypair. 409 if they exist. |
| `user banish` | `PATCH /api/admin/user/banish` | Toggle the banished flag. |
| `user generate-code` | `POST /api/admin/user/code` | Generate a login code for an existing player. 404 if they do not exist. |

Flags: `-p, --player <name>`, `-g, --game <minecraft>`. `user banish` also takes `-b, --banish <true|false>` (default `true`). `user generate-code` also takes `-d, --duration <secs>`.

## Permissions

Requires an active admin identity. Valid strings: `audio_upload`, `audio_delete`, `admin`.

| Command | Endpoint | Does |
|---|---|---|
| `permission allow` | `PUT /api/admin/permission` | Record an explicit allow. |
| `permission deny` | `PUT /api/admin/permission` | Record an explicit deny. |
| `permission clear` | `DELETE /api/admin/permission` | Remove the override. The config default applies again. |
| `permission list` | `GET /api/admin/permission/{game}` | List explicit overrides. |

Flags: `-p, --player <name>`, `-g, --game <minecraft>`, and `--permission <name>` for all but `list`.

`clear` is not `deny`. It removes the override.

Empty output from `list` means no overrides. The defaults govern that player entirely.

## Peering

Over HTTPS with mTLS. Requires an active admin identity.

| Command | Endpoint | Does |
|---|---|---|
| `relay peerlink` | `GET /api/admin/relay/peerlink` | Print this server's peer link, its node id, and a `peer` block to paste into the other server's `config.hcl`. |
| `relay worlds` | `GET /api/admin/relay/worlds` | List relay worlds with player counts. |

`relay peerlink` takes `-l, --label <label>`, the block label rendered in the printed example. Default `bvc-server`. Use what the far side will call this server.

`relay peerlink` exits non-zero on a server with no `peer` block in `config.hcl`. The peer endpoint binds only when one exists.

`relay worlds` prints nothing until a player joins a relay world.

See [peering](/wiki/reference/peering/).

## Bootstrapping a fresh deployment

```bash
# Server host, config.hcl available
bvc admin generate-code -p Alice -g minecraft -d 3600
# -> Code: ABCD1234

# Anywhere with network access
bvc login --code ABCD1234

# Server host
bvc admin bootstrap -p Alice -g minecraft

# Alice now has admin
bvc whoami
bvc user add -p Bob -g minecraft
bvc user generate-code -p Bob -g minecraft
```

See [players and permissions](/wiki/server/players-and-permissions/).
