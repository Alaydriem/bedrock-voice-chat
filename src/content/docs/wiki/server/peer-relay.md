---
title: Peer relay
description: Deploying the iroh relay BVC peers connect through when no direct path exists.
sidebar:
  label: Peer relay
  order: 9
---

:::caution[Work in progress]
Peering is under active development. Relay deployment and its access control change between releases. Treat everything here as provisional.
:::

The peer relay is an [iroh relay](https://github.com/n0-computer/iroh) that Bedrock Voice Chat peers connect through when they have no direct path to each other. It is the upstream `iroh-relay` server, pinned and packaged. There is no Bedrock Voice Chat code in it.

Deployment files are in `relay/n0/` in the BVC repository.

## When you need one

Most deployments do not. Two BVC servers on one host, and two hosts on one local network, reach each other at an address the peer link already carries.

Deploy a relay when two peers have no direct path.

## Scope

It forwards opaque, end-to-end encrypted frames between endpoints. It holds no BVC key material, terminates no BVC session, and cannot read voice or positions.

Peers that can reach each other directly stop using it after the handshake.

## Deploy

1. Point a DNS record at the host.
2. Set `hostname` in `relay.toml` to that name. The certificate is issued for it. It must match what peers dial.
3. Start it:

   ```bash
   docker compose up -d --build
   ```

## Ports

| Port | Purpose |
|---|---|
| 80/tcp | Let's Encrypt HTTP-01 challenge |
| 443/tcp | Relay traffic |
| 443/udp | QUIC address discovery |

All three must be reachable. Without 443/udp every peer pair relays forever instead of finding a direct path.

## Wiring a BVC server

In each server's `config.hcl`:

```hcl
server {
    peer_relay_url = "https://relay.example.com"
}
```

Leave it unset and a peer link is reachable only at an address the far side already holds. See [`server.peer_relay_url`](/wiki/reference/configuration/#serverpeer_relay_url).

## Version pinning

`IROH_VERSION` must match the `iroh` version in `relay/lib/Cargo.toml`. The relay protocol is tied to the iroh release. A mismatch fails when a peer tries to relay, not when the image builds.

`build-relay-docker.yml` reads the version from the lockfile.

## Peering

See [peering](/wiki/reference/peering/).
