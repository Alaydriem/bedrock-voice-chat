---
title: PumpkinMC on Nodecraft
description: Run Bedrock Voice Chat on a Nodecraft PumpkinMC server.
# Draft until BVC supports PumpkinMC. Drafts are left out of production builds
# and the sidebar. When this goes live, remove `draft` and add a PumpkinMC row
# to the table in platforms/nodecraft/index.md.
draft: true
sidebar:
  label: PumpkinMC
  order: 4
---

This guide installs Bedrock Voice Chat on a Nodecraft [PumpkinMC](https://pumpkinmc.org/) server.

## Before you start

You need:

- A Nodecraft Pro server. See [Nodecraft](/wiki/platforms/nodecraft/).
- A domain name, with DNS at Cloudflare or another [supported provider](/wiki/server/tls/).
- A Cloudflare API token for that zone, with DNS edit permission. See [TLS certificates](/wiki/server/tls/).

:::tip
You **must** use a Pro server. Lite servers suspend themselves after a period of inactivity, which causes problems with Bedrock Voice Chat.
:::

## Install PumpkinMC

<!-- TODO: Nodecraft server setup steps for PumpkinMC. -->

<div class="shot"><span>Nodecraft Browse Mod to Install dialog filtered to PumpkinMC</span></div>

## Install Bedrock Voice Chat

<!-- TODO: what to download, where it goes, and the config file path and
     format. Verify against the BVC source once PumpkinMC support exists. -->

<div class="shot"><span>Nodecraft File Manager, PumpkinMC plugin directory with the BVC plugin</span></div>

## Point your domain at the server

1. In the Pro Panel, select **Overview**. Copy the **Dedicated IP**.
2. At your DNS provider, add an `A` record for your BVC name, for example `bvc.example.com`, with the Dedicated IP as its value.
3. In Cloudflare, set the record's proxy status to **DNS only**.

## Connect

Your players sign in with `https://bvc.example.com:28280`. Use your domain. See [signing in](/wiki/player/signing-in/).

<!-- TODO: confirm the port once the PumpkinMC config exists. -->

<div class="window"><img src="/assets/wiki/nodecraft-login.jpg" alt="BVC login window with bvc.example.com:28280 in the server address field" /></div>

## Next steps

<!-- TODO: admin bootstrap command for PumpkinMC. -->

1. Send your players to [Downloads](/wiki/start/downloads/). They can then sign in.
