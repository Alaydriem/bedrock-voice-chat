---
title: Aternos and no-net hosts
description: Run BVC on Aternos and other hosts that cannot enable @minecraft/server-net.
sidebar:
  label: Aternos
  order: 4
---

Aternos and similar hosts are supported as of 1.0.0-beta.15.

These hosts do not let you enable `@minecraft/server-net`, so BVC uses the no-net Addon: your client becomes the Bedrock server your game connects to, proxies through to the real host, and reads positions off the wire.

Read the [limitations](/wiki/platforms/#realms-and-aternos) first. Support lags every Minecraft release, and a host that auto-updates will break voice on update day.

## Before you start

1. Install and configure the [BVC server](/wiki/server/installation/). Not optional.
2. Install the [client app](/wiki/start/downloads/).
3. Build the no-net world following the [no-net Addon](/wiki/server/nonet-addon/) guide.

## Upload the world

1. Find the `.mcworld` you exported and change the extension to `.zip`.
2. Upload it to Aternos.

   <div class="shot"><span>Aternos world upload page</span></div>

3. Set the Aternos server version to match the Addon's supported Minecraft version. These Addons are not backwards compatible — a mismatch fails outright.

   <div class="shot"><span>Aternos server version selector</span></div>

Check [version support](/wiki/platforms/version-support/) for what to pick.

## Connect

1. Sign into the BVC client against the server from step 1.
2. Settings cog, bottom left. Then **Proxy Connect** in the sidebar. First visit asks you to sign in again.
3. Add a server with the button at the top: your Aternos domain and port.

   <div class="shot"><span>Proxy Connect add-server dialog with an Aternos address</span></div>

4. Connect. You get connection details back.

   <div class="shot"><span>Proxy Connect showing connection information for a connected server</span></div>

## Open Minecraft

**Desktop and mobile** — add a server manually:

- Your BVC server's hostname on port 19132, or
- the IP shown under **From another device on your LAN (phone, tablet)**.

**Console** — set both primary and secondary DNS on the console to your BVC server's IP, then pick **Hive** under Featured Servers.

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

## Do not enter the Aternos address directly

Connecting straight to your Aternos server bypasses BVC. Minecraft works, the app looks fine, and no voice passes.

You must go in through Proxy Connect.

## Getting support sooner

Patreon supporters and YouTube members get early access to builds and packs, which usually means working against a new Bedrock version before general release. See [supporting BVC](/wiki/start/supporting-bvc/).

Or run Bedrock Dedicated Server, which needs none of this.
