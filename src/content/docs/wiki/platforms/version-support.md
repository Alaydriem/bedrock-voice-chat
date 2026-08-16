---
title: Bedrock version support
description: Which Minecraft Bedrock versions BVC can speak, and why support lags each release.
sidebar:
  label: Version support
  order: 2
---

Bedrock Voice Chat Connect speaks Bedrock's own network protocol. Bedrock Voice Chat therefore has to understand the exact protocol version your server runs. Below is every version it supports.

This matters for Realms, Aternos, and any other host using the no-net Addon.

:::note
None of this applies to a Bedrock Dedicated Server or a Java server. The Addon reports positions over HTTP, and the Bedrock protocol version is irrelevant there.
:::

<!-- generated:version-table start -->

## Supported Bedrock versions

| Minecraft Bedrock | Protocol |
|---|---|
| 1.26.0 | `v924` |
| 1.26.12 | `v944` |
| 1.26.20 | `v975` |
| 1.26.30 | `v1001` |
| 1.26.40 | `v2168` |

**1.26.40 is the newest supported release.**

## Not supported

These protocols are recognised but this build has no codec for them. The
no-net Addon and Bedrock Voice Chat Connect will not work against a server running one.

| Minecraft Bedrock | Protocol |
|---|---|
| 1.16.201 | `v422` |
| 1.16.220 | `v431` |
| 1.19.30 | `v554` |
| 1.20.61 | `v649` |
| 1.21.50 | `v766` |
| 1.21.90 | `v786` |

## Preview builds

A codec exists, but no public Bedrock release speaks these yet.

| Minecraft Bedrock | Protocol |
|---|---|
| 1.26.50 | `v2169` |

<!-- generated:version-table end -->

## Why Bedrock Voice Chat Connect Support Lags Minecraft Releases

Mojang does not publish the Bedrock packet protocol specification on a consistent schedule. Nearly every Berock release changes it, and each change needs to be reverse-engineered from the protocol and re-generated before Bedrock Voice Chat can speak the new version, followed by an update to the apps through their respective stores.

Two things follow from that:

- The no-net Addon is locked to a protocol. An Addon built for one Bedrock version will not work against another. Match your server version to the Addon you installed.
- If your host auto-updates Minecraft, expect voice to stop working on update day until a matching Bedrock Voice Chat build ships.

:::tip
Running your own Bedrock Dedicated Server avoids both. BDS lets you choose when to update, and the position relay does not depend on the packet protocol.

Patreon supporters and YouTube members get early access to builds and packs, which usually means working against a new Bedrock version before general release. See [supporting BVC](/wiki/start/supporting-bvc/).
:::

## Advertised version

When connecting through Bedrock Voice Chat Connect you can pin the protocol BVC advertises, per server, in the client's server list. Auto mirrors whatever the real backend reports and is correct in almost every case. Leave it there unless you have a reason not to.

<div class="shot"><span>Bedrock Voice Chat Connect server entry with the advertised-version dropdown open</span></div>

---

<small>Generated from the protocol matrix emitted by <code>bedrock-protocol-rs</code>. </small>
