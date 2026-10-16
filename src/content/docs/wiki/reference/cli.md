---
title: CLI
description: "Command-line reference for the server executable: identities, users, permissions, and bootstrapping."
sidebar:
  label: CLI
  order: 4
---

The Bedrock Voice Chat server executable is also a command-line tool. Use it to manage identities, users and permissions on the local server or on a remote server.

Available since beta.11.

## Global flags

| Flag | Env | Default | Notes |
|---|---|---|---|
| `-c, --config-file <path>` | None | `config.hcl` | Used only by `server` and `admin *`. |
| `--server-url <url>` | `BVC_SERVER` | `https://127.0.0.1:3000` | The server that `login` connects to. Set it to your server's URL. Other commands read the URL from the stored identity. |
| `--identity <gamertag:game>` | `BVC_IDENTITY` | None | The stored identity to use when there is more than one. |

## Identity

| Command | Auth | Transport | Result |
|---|---|---|---|
| `login` | None | `POST /api/auth/code` | Exchanges a one-time code for an mTLS certificate bundle and stores it. |
| `logout` | mTLS | Local | Deletes the active stored identity. |
| `whoami` | mTLS | `GET /api/auth/introspect` | Prints the identity, server URL, certificate expiry, and effective permissions. |

`login` takes one flag, `--code <code>`. If you do not give the flag, `login` asks for the code on stdin.

`login` needs only the code. The server finds the player from the code and returns the gamertag and the game.

## Server

| Command | Result |
|---|---|
| `server` | Starts the BVC server (HTTP and QUIC). Reads `--config-file`. |

## Bootstrap

These commands connect directly to the database. Run them on the server host, with `config.hcl` available. Use them when no admin exists yet.

| Command | Flags | Result |
|---|---|---|
| `admin bootstrap` | `-p, --player <name>`; `-g, --game <minecraft>` | Gives `admin` to the player. Creates the player record if it does not exist. |
| `admin generate-code` | `-p, --player <name>`; `-g, --game <minecraft>`; `-d, --duration <secs>`; `--ephemeral <true\|false>` | Makes a one-time login code. Creates the player record if it does not exist. |

`-d` has a default of 3600.

`--ephemeral` has a default of `true`: the code is used up when it is redeemed. Pass `--ephemeral false` to make a code that stays valid until it expires and can be used more than one time.

## Users

These commands use HTTPS with mTLS. They need an active admin identity (`whoami` shows `admin`).

| Command | Endpoint | Result |
|---|---|---|
| `user add` | `POST /api/admin/user` | Creates a player with a certificate and a keypair. Returns 409 if the player exists. |
| `user banish` | `PATCH /api/admin/user/banish` | Sets or clears the banished flag. |
| `user generate-code` | `POST /api/admin/user/code` | Makes a login code for an existing player. Returns 404 if the player does not exist. |

Flags: `-p, --player <name>` and `-g, --game <minecraft>`. `user banish` also takes `-b, --banish <true|false>` (default `true`). `user generate-code` also takes `-d, --duration <secs>` (default 3600) and `--ephemeral <true|false>` (default `true`).

## Permissions

These commands need an active admin identity. Valid permission names: `audio_upload`, `audio_delete`, `admin`.

| Command | Endpoint | Result |
|---|---|---|
| `permission allow` | `PUT /api/admin/permission` | Sets an allow override. |
| `permission deny` | `PUT /api/admin/permission` | Sets a deny override. |
| `permission clear` | `DELETE /api/admin/permission` | Removes the override. The config default then applies. |
| `permission list` | `GET /api/admin/permission/{game}` | Lists the overrides. |

Flags: `-p, --player <name>` and `-g, --game <minecraft>`. All commands except `list` also take `--permission <name>`.

If `list` prints nothing, the player has no overrides. Only the config defaults apply.

## Peering

These commands use HTTPS with mTLS. They need an active admin identity.

| Command | Endpoint | Result |
|---|---|---|
| `relay peerlink` | `GET /api/admin/relay/peerlink` | Prints this server's peer link, its node ID, and a `peers` block to paste into the other server's `config.hcl`. |
| `relay worlds` | `GET /api/admin/relay/worlds` | Lists relay worlds with player counts. |

`relay peerlink` takes `-l, --label <label>`. This is the block label in the printed example. The default is `bvc-server`. Set the label to the name that the other server uses for this server.

`relay peerlink` exits with a non-zero code on a server that has no `peers` block in `config.hcl`. The peer endpoint binds only when a `peers` block exists.

`relay worlds` prints nothing until a player joins a relay world.

See [peering](/wiki/reference/peering/).

## Bootstrapping a new deployment

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
