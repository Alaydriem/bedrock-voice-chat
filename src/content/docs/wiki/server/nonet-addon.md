---
title: No-net Addon
description: Build and install the Addon for Realms, Aternos, and hosts without @minecraft/server-net.
sidebar:
  label: No-net Addon
  order: 6
---

Some hosts do not let you enable `@minecraft/server-net`, so the Addon cannot make HTTP requests and cannot report positions the normal way. Realms and Aternos are both in this category.

The no-net Addon works differently: your BVC client becomes the Bedrock server your game connects to, proxies packets through to the real host, and reads positions off the wire.

This page is the part that is identical for every such host. The host-specific steps are in [Realms](/wiki/platforms/realms/) and [Aternos](/wiki/platforms/aternos/).

## Before you start

Install and configure the [BVC server](/wiki/server/installation/) first.

:::caution
Bedrock Voice Chat Server is _not_ an optional component. The no-net path still needs this.
:::

Install the [client app](/wiki/start/downloads/) too, since part of this happens there.

## Read the limitations first

This approach is constrained by what Bedrock's Addon system allows, and the constraints are real:

- The Addon is **locked to a protocol version**. One built for a given Bedrock version will not work against another.
- Support **lags every Minecraft release**. Mojang does not document the packet protocol and it changes each update, so the work cannot start until the update ships. There is no guaranteed day-one support.
- A host that auto-updates Minecraft **will break voice** on update day.

Running your own Bedrock Dedicated Server avoids all of it. See [version support](/wiki/platforms/version-support/) for what currently works.

## Build the world

1. Download the `no-net.mcaddon` package — GitHub Releases under the `mods-` tag, or CurseForge. It must be the `no-net` variant; the full Addon will not work here.

2. Double-click it to install into your local Minecraft Bedrock client.

3. Create a new world. Under **Behavior Packs**, enable the Bedrock Voice Chat Realms + No-Net pack.

   <div class="shot"><span>Behavior Packs list with the Realms + No-Net pack enabled</span></div>

4. Under **Experiments**, enable **Beta APIs**.

   <div class="shot"><span>Experiments toggle showing Beta APIs enabled</span></div>

5. Create the world, then exit it immediately.

6. Open the world's settings and export it.

   <div class="shot"><span>World settings with the Export World option at the bottom</span></div>

Save it somewhere you can find.

## Then

The world you just built gets uploaded to your host, and the client gets pointed at it. That part differs:

- **[Realms](/wiki/platforms/realms/)** — upload to the Realm, connect through Realms Connect.
- **[Aternos and other no-net hosts](/wiki/platforms/aternos/)** — rename to `.zip`, upload, connect through Proxy Connect.

## Match the versions

Whatever host you are on, its Minecraft version must match the one the Addon supports. These Addons are not backwards compatible — a mismatch fails, it does not degrade.

Check [version support](/wiki/platforms/version-support/) before picking a server version.

## Console players

Consoles cannot enter a custom server address, so they reach BVC through DNS. That needs the DNS responder enabled on your BVC server, and it is **off by default**:

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
