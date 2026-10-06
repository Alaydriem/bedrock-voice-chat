---
title: Nodecraft
description: Run Bedrock Voice Chat on a Nodecraft game server, for Fabric, PaperMC, and Bedrock.
sidebar:
  label: Overview
  order: 1
---

[Nodecraft](https://nodecraft.com/r/alaydriem) provides profesional Minecraft server hosting that works with Bedrock Voice Chat for both Java and Bedrock servers.

:::tip[Nodecraft discount]
Sign up through [nodecraft.com/r/alaydriem](https://nodecraft.com/r/alaydriem) to get the best available discount. The link also supports Bedrock Voice Chat development.
:::

## Requirements

- A Nodecraft server.
- A domain name. Bedrock Voice Chat does not work with an IP address or with the Nodecraft host name. See [TLS certificates](/wiki/server/tls/).
- A DNS provider that BVC can use to issue a certificate. Cloudflare is the simplest. See [TLS certificates](/wiki/server/tls/).

## Pick your server type

| Server type | BVC server | Guide |
|---|---|---|
| **Fabric** | Embedded in the mod | [Fabric on Nodecraft](/wiki/platforms/nodecraft/fabric/) |
| **PaperMC** | Embedded in the plugin | [PaperMC on Nodecraft](/wiki/platforms/nodecraft/papermc/) |

The Java guides use the mod's [embedded mode](/wiki/server/java-mod/#integrated-or-external-bvc-server). The BVC server runs in the Minecraft server's JVM and uses the Nodecraft server's ports.

## Support

For problems with the Nodecraft panel, billing, or the server itself, contact [Nodecraft support](https://nodecraft.com/support). For problems with Bedrock Voice Chat, see [server troubleshooting](/wiki/server/troubleshooting/) or [report an issue](/wiki/start/report-an-issue/).
