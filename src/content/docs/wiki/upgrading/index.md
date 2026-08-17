---
title: Upgrading
description: Moving an existing Bedrock Voice Chat deployment to a newer release.
sidebar:
  label: Overview
  order: 1
---

Bedrock Voice Chat releases the server, the client, and the Minecraft mods together under one version. An upgrade moves all three.

## Version pairing

Run the same release everywhere. Voice carries its own protocol version, separate from the release version. The client compares the major and minor of its protocol against the server's and refuses the session when they differ. Patch releases do not change the wire format.

| Release | Voice protocol |
|---|---|
| 1.0.0-beta.21 | 3.0.0 |
| 1.0.0-beta.20 | 2.1.0 |

On the server list, a client behind the server shows **Update needed**. A client ahead of the server shows **Server is older**. Both states block the connection.

## Procedure

1. Back up the database.
2. Stop the server.
3. Apply the configuration changes listed on the release page.
4. Replace the binary, or pull the new container image.
5. Start the server. Database migrations run at startup.
6. Read the startup log before letting players connect.
7. Update the Addon and any Java mod to the same release.

:::caution
Apply configuration changes **before** the first start on the new version. The server ignores keys it does not recognise, and a removed block leaves no trace in the log.
:::

## Release guides

- [Upgrading to 1.0.0-beta.21](/wiki/upgrading/beta-21/)
