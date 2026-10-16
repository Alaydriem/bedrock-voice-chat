---
title: Bedrock version support
description: The Bedrock versions that Bedrock Voice Chat Connect supports, and why support comes after each release.
sidebar:
  label: Version support
  order: 2
---

Bedrock Voice Chat Connect uses the Bedrock network protocol. Connect works only with the protocol versions in the table below.

This applies to Realms, Aternos and other hosts that use the no-net Addon.

:::note
This does not apply to a Bedrock Dedicated Server or a Java server. On these servers, the Addon or the mod sends positions over HTTP, and the Bedrock protocol version has no effect.
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
| 1.26.45 | `v2169` |
| 1.26.50 | `v2192` |
| 1.26.50 | `v2193` |

**1.26.50 is the newest supported release.**

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

<!-- generated:version-table end -->

## Release delay

Mojang does not document the Bedrock packet protocol. Nearly every Bedrock release changes the protocol. For each change, the BVC developers reverse-engineer the new protocol and update BVC. Then they release new apps through each app store.

- The no-net Addon is locked to one protocol version. An Addon for one Bedrock version does not work with a different version. Use the server version that matches the Addon that you installed.
- If your host updates Minecraft automatically, voice chat stops on the day of the update. It works again when a matching Bedrock Voice Chat version is available.

:::tip
On your own Bedrock Dedicated Server, these limits do not apply. You choose when to update, and the Addon sends positions over HTTP. A protocol change does not affect it.

Patreon supporters and YouTube members get early access to builds and packs. They usually get support for a new Bedrock version before the general release. See [supporting BVC](/wiki/start/supporting-bvc/).
:::

## Advertised version

Each server in the app's server list has an advertised version setting. It sets the protocol version that Bedrock Voice Chat Connect reports to Minecraft. The default, Auto, uses the version that the Realm or host reports. Keep Auto unless a server does not connect with it.
