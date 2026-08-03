---
title: Realms
description: Run BVC on a Minecraft Realm using the no-net Addon and Realms Connect.
sidebar:
  label: Realms
  order: 3
---

Realms are supported as of 1.0.0-beta.15.

A Realm cannot enable `@minecraft/server-net`, so BVC uses the no-net Addon: your client becomes the Bedrock server your game connects to, proxies through to the Realm, and reads positions off the wire.

Read the [limitations](/wiki/platforms/#realms-and-aternos) before committing. Realms auto-update, and an update breaks voice until a matching BVC build ships.

## Before you start

1. Install and configure the [BVC server](/wiki/server/installation/). Not optional — the no-net path still needs one.
2. Install the [client app](/wiki/start/downloads/).
3. Build the no-net world following the [no-net Addon](/wiki/server/nonet-addon/) guide.

## Upload to the Realm

Create your world on the Realm from the world you built, then exit the Realm once it has saved.

## Connect

Realms use Nethernet, which has a connect cooldown. If you disconnect from Realms Connect you may wait around five minutes before reconnecting while the Realm is idle.

1. Sign into the BVC client against the server from step 1.
2. Settings cog, bottom left. Then **Realms Connect** in the sidebar. First visit asks you to sign in again.
3. Select your Realm.

<div class="shot"><span>Realms Connect page with a Realm selected</span></div>

## Open Minecraft

**Desktop and mobile** — add a server manually:

- Your BVC server's hostname on port 19132, or
- the IP shown under **From another device on your LAN (phone, tablet)**.

**Console** — set both primary and secondary DNS on the console to your BVC server's IP, then pick **Hive** under Featured Servers. You are redirected to your Realm.

Console needs the DNS responder enabled on your BVC server, and it is off by default:

```hcl
server {
    bedrock {
        dns {
            enabled = true
        }
    }
}
```

See [console and mobile](/wiki/platforms/console-and-mobile/).

## Do not use the Realms tab

Joining through Minecraft's own **Realms** tab bypasses BVC completely. The game works, the app looks connected, and nobody hears anyone.

You must go in through Realms Connect.

<div class="shot"><span>Connection information shown after selecting a Realm</span></div>
