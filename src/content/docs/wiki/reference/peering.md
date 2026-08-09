---
title: Peering
description: Linking two BVC servers so voice carries across both. Early access.
sidebar:
  label: Peering
  order: 6
---

Peering links two BVC servers. Players whose realms overlap hear each other across both.

**Early access.** It works. It is still being hardened, and the configuration shape may change between releases. Not recommended for production communities.

## How it works

Discovery is decentralised. Servers announce themselves in-realm instead of registering with a broker. There is no central relay role and no third party in the path.

A server offers a link, the peer redeems a code, and the link is established. Voice traffic then flows between the two.

Idle links are swept and re-established as needed.

## Permission

Gated by `peer_link`, which defaults to off.

```bash
bvc permission allow -p <player> -g minecraft --permission peer_link
```

Or server-wide:

```hcl
permissions {
    defaults = {
        peer_link = true
    }
}
```

## Configuration

Under `server.features.relay`:

| Key | Default | Description |
|---|---|---|
| `announce_interval_secs` | `60` | Seconds between self-announce cycles. |
| `orchestration_interval_secs` | `5` | Seconds between offer, idle-sweep, and reconnect-grace cycles. |
| `idle_timeout_secs` | `300` | Seconds a link may idle before it is closed. |

```hcl
server {
    features {
        relay {
            announce_interval_secs = 60
        }
    }
}
```

The defaults suit production. Lowering them buys responsiveness at the cost of traffic.

## Endpoints

Mounted only when the relay plane is active:

| Endpoint | Does |
|---|---|
| `POST /api/relay/offer` | Offer a peer link. |
| `POST /api/relay/peer-redeem` | Redeem a peering code. |
| `POST /api/relay/peer-link` | Establish the link. |

## Before you turn it on

Peering means players on another operator's server hear yours, and yours hear theirs. Their moderation becomes your problem, and yours becomes theirs.

Grant `peer_link` deliberately.
