---
title: Console and mobile
description: Using BVC on Xbox, PlayStation, Switch, and mobile devices.
sidebar:
  label: Console and mobile
  order: 6
---

There is no on-console BVC client and there will not be one. Console players run the mobile app on a phone or tablet next to the console.

Your position comes from the game server, so the console does nothing special — the app knows where you are without being installed on it.

## Get the app

- **Android** — [Google Play](https://play.google.com/store/apps/details?id=com.alaydriem.bvc.client), open beta
- **iOS / iPadOS** — [TestFlight](https://testflight.apple.com/join/JSG7bVqC)

Sign in with the same Xbox account you play on.

## On a Bedrock Dedicated Server

Nothing console-specific. Join the server as usual, sign into the app, done.

## On Realms or Aternos

These go through Realms Connect or Proxy Connect, and a console cannot type a custom server address. Instead you point the console's DNS at your BVC server, which answers for a featured-server hostname and redirects the console to the right place.

### Operator: enable the DNS responder

**It is off by default.** Without this, console players cannot connect at all.

```hcl
server {
    bedrock {
        dns {
            enabled = true
        }
    }
}
```

Then:

- Publish **53/UDP** on the BVC server. In Docker that is `-p 53:53/udp`.
- Binding port 53 needs elevated privileges, or `CAP_NET_BIND_SERVICE` on Linux. In a container, add the capability.
- Port 53 must be reachable from the console's network.

Full options in the [configuration reference](/wiki/reference/configuration/#serverbedrockdns).

### Player: point the console at it

Set **both** primary and secondary DNS on the console to your BVC server's IP. Setting only the primary leaves the console free to fall back to the secondary, which defeats it.

Then open Minecraft, go to **Servers**, and pick **Hive** under Featured Servers. You are redirected to your Realm or Aternos server.

Do not enter your Realm or Aternos address directly. That bypasses BVC — Minecraft works and nobody hears anyone.

## Mobile as a player, not a console companion

Everything above is about consoles. A phone or tablet playing Minecraft itself is simpler: it can enter a custom server address, so it follows the desktop instructions on the [Realms](/wiki/platforms/realms/) and [Aternos](/wiki/platforms/aternos/) pages.

## Recording

Not available on mobile. Phones and tablets lack the disk throughput for multi-track capture. Record on a desktop instead — see [Recording](/wiki/creator/recording/).
