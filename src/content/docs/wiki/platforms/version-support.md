---
title: Bedrock version support
description: Which Minecraft Bedrock versions BVC can speak, and why support lags each release.
sidebar:
  label: Version support
  order: 2
---

This matters for Realms, Aternos, and any other host using the no-net Addon or Proxy Connect. Those paths speak Bedrock's own network protocol. BVC has to understand the exact protocol your server runs.

None of this applies to a normal Bedrock Dedicated Server or a Java server. The Addon reports positions over HTTP, and the Bedrock protocol version is irrelevant.

<!-- generated:version-table start -->

## Supported Bedrock versions

| Minecraft Bedrock | Protocol |
|---|---|
| 1.26.0 | `v924` |
| 1.26.12 | `v944` |
| 1.26.20 | `v975` |
| 1.26.30 | `v1001` |

**1.26.30 is the newest supported release.**

## Not supported

These protocols are recognised but this build has no codec for them. The
no-net Addon and Proxy Connect will not work against a server running one.

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

## Why support lags a Minecraft release

Mojang does not publish a specification for the Bedrock packet protocol and does not commit to a schedule. Every Bedrock release changes it, and each change has to be reverse-engineered by hand before BVC can speak the new version. The work cannot start until the update ships. There is no day-one support.

Two things follow from that:

- The no-net Addon is locked to a protocol. An Addon built for one Bedrock version will not work against another. Match your server version to the Addon you installed.
- If your host auto-updates Minecraft, expect voice to stop working on update day until a matching BVC build ships.

Running your own Bedrock Dedicated Server avoids both. BDS lets you choose when to update, and the position relay does not depend on the packet protocol.

Patreon supporters and YouTube members get early access to builds and packs, which usually means working against a new Bedrock version before general release. See [supporting BVC](/wiki/start/supporting-bvc/).

## Advertised version

When connecting through Proxy Connect you can pin the protocol BVC advertises, per server, in the client's server list. Auto mirrors whatever the real backend reports and is correct in almost every case. Leave it there unless you have a reason not to.

<div class="shot"><span>Proxy Connect server entry with the advertised-version dropdown open</span></div>

---

<small>Generated from the protocol matrix emitted by <code>bedrock-protocol-rs</code>. Tracks the shipped build.</small>
