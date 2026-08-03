---
title: Peering
description: Linking two BVC servers so voice carries across both. Early access.
sidebar:
  label: Peering
  order: 6
---

Peering links two BVC servers so players whose realms overlap can hear each other across both.

**Early access.** It works, it is still being hardened, and the configuration shape may change between releases. Not recommended for production communities yet.

## How it works

Discovery is decentralised. Servers announce themselves in-realm rather than registering with a broker, so there is no central relay role and no third party in the path.

From there: a server offers a link, the peer redeems a code, and the link is established. Once up, voice traffic flows between the two.

Links are swept when idle and re-established as needed, so a quiet peer does not hold a connection open indefinitely.

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

The defaults suit production. These keys exist mainly so integration tests can shorten the cycles, and lowering them in production buys responsiveness at the cost of traffic.

## Endpoints

Mounted only when the relay plane is active:

| Endpoint | Does |
|---|---|
| `POST /api/relay/offer` | Offer a peer link. |
| `POST /api/relay/peer-redeem` | Redeem a peering code. |
| `POST /api/relay/peer-link` | Establish the link. |

## Before you turn it on

Peering means players on another operator's server can hear yours and vice versa. You are extending trust to whoever runs the other end — their moderation becomes your problem and yours becomes theirs.

Grant `peer_link` deliberately. The default is off for this reason.
