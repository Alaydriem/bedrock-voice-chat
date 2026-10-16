---
title: Upgrading
description: Moving an existing Bedrock Voice Chat deployment to a newer release.
sidebar:
  label: Overview
  order: 1
---

Bedrock Voice Chat releases the server, the client and the Minecraft mods together, with one version number. Upgrade all three to the same version.

## Version pairing

Use the same release everywhere. The voice protocol has its own version number, which is not the release version. The client compares the major and minor numbers of its voice protocol with those of the server. If they are different, the client refuses the session. A change to only the patch number does not affect compatibility.

| Release | Voice protocol |
|---|---|
| 1.0.0-beta.21 | 3.0.0 |
| 1.0.0-beta.20 | 2.1.0 |

On the server list, a client that is older than the server shows **Update needed**. A client that is newer than the server shows **Server is older**. In both cases, the client cannot connect.

## Procedure

1. Back up the database.
2. Stop the server.
3. Make the configuration changes listed in the release guide.
4. Replace the binary, or pull the new container image.
5. Start the server. Database migrations run at startup.
6. Read the startup log before you let players connect.
7. Update the Addon and any Java mod to the same release.

:::caution
Make the configuration changes **before** the first start on the new version. The server ignores most unknown keys and writes nothing to the log about them. Some removed keys stop the server. The release guide lists them.
:::

## Release guides

- [Upgrading to 1.0.0-beta.21](/wiki/upgrading/beta-21/)
