---
title: Peering
description: Linking two BVC servers so voice carries across both. Work in progress.
sidebar:
  label: Peering
  order: 6
---

:::caution[Work in progress]
Peering is under active development. The configuration shape, the CLI, and the wire protocol all change between releases. Peering protocol is subject to change until the spec is finalized

Peering links two Bedrock Voice Chat servers together, letting players whose worlds overlap hear each other across both.

Not for production communities.

## Model

Peering is declared. There is no discovery, no broker, and no announce cycle. Each operator writes the other's peer link into their own `config.hcl`. A node absent from that list is refused at connect.

The declaration is the authorization. There is no peering permission.

## Link two servers

Run on server A:

```bash
bvc relay peerlink --label server-b
```

It prints the peer link, the node id, and a block to paste. Put that block in server B's `config.hcl`. Then run the same command on server B and paste its block into server A's.

Restart both.

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
| `peerlink` | — | The link the far side printed. **Required.** |
| `worlds` | `[]` | Narrowing filter over the worlds the peer declares. |
| `capabilities` | `["carry_speakers"]` | What the peer may do on the link. |

Full descriptions in the [configuration reference](/wiki/reference/configuration/#serverpeers).

## Reaching a peer

A peer link carries the paths to reach its server. Two servers on one host, or two hosts on one local network, need nothing else.

Set [`server.peer_relay_url`](/wiki/reference/configuration/#serverpeer_relay_url) when no direct path exists. See [deploying a peer relay](/wiki/server/peer-relay/).

## Diagnostics

| Command | Does |
|---|---|
| `bvc relay peerlink` | Print this server's peer link and node id. |
| `bvc relay worlds` | List the relay worlds this server hosts, with player counts. |

`bvc relay worlds` lists nothing until a player joins a relay world.

Both read the admin API over mTLS and need an active admin identity. See the [CLI reference](/wiki/reference/cli/).

## Before you turn it on

Peering means players on another operator's server hear yours, and yours hear theirs. Their moderation becomes your problem, and yours becomes theirs.

Declare a peer deliberately.
