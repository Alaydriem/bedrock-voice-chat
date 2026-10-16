---
title: "Peering"
description: "Linking two BVC servers so voice carries across both. Work in progress."
source: https://www.bedrockvoicechat.com/wiki/reference/peering/
---
:::caution[Work in progress]
Peering is in development. The configuration, the CLI, and the protocol change between releases. Do not use peering on a production community.
:::

Peering links two Bedrock Voice Chat servers. Players whose worlds overlap can then hear each other across both servers.

## Peer declaration

Servers do not find each other automatically. Each operator adds the other server's peer link to their own `config.hcl`. A server refuses a connection from any server that is not in that list.

A server that is listed in `server.peers` can connect. No player permission controls peering.

## Link two servers

On server A, run:

```bash
bvc relay peerlink --label server-a
```

The command prints the peer link, the server's node ID, and a `peers` block. Paste the block into server B's `config.hcl`. Then run the same command on server B with `--label server-b`, and paste its block into server A's `config.hcl`.

Restart both servers.

Server A's `config.hcl`:

```hcl
server {
    peers "server-b" {
        peerlink = "<the value server B printed>"
    }
}
```

## Configuration

| Key | Default | Description |
|---|---|---|
| `peerlink` | None | The peer link that the other server printed. **Required.** |
| `worlds` | `[]` | Limits the link to some of the worlds that the peer declares. |
| `capabilities` | `["carry_speakers"]` | The actions that the peer can do on the link. |

See the [configuration reference](/wiki/reference/configuration/#serverpeers) for full descriptions.

## Network access

A peer link contains the addresses of its server. Two servers on one host, or two hosts on one local network, need nothing more.

Over the internet, the two servers connect directly. No relay forwards the traffic. No voice or position data goes through Bedrock Voice Chat infrastructure.

At least one server must accept incoming connections. If both servers are behind NAT, forward [`server.peer_port`](/wiki/reference/configuration/#serverpeering-and-serverpeer_port) (UDP `28284` by default) on one of them. If neither server can accept connections, peering does not work. This usually happens when both servers are behind carrier-grade NAT.

## Diagnostics

| Command | Result |
|---|---|
| `bvc relay peerlink` | Prints this server's peer link and node ID. |
| `bvc relay worlds` | Lists the relay worlds that this server hosts, with player counts. |

`bvc relay worlds` lists nothing until a player joins a relay world.

Both commands use the admin API over mTLS and need an active admin identity. See the [CLI reference](/wiki/reference/cli/).

## Moderation

Players on the peer server hear your players, and your players hear them. The moderation of each operator affects both communities. Declare a peer only when you trust the other operator's moderation.