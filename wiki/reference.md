---
title: "Reference"
description: "Configuration, environment variables, CLI, and the HTTP API."
source: https://www.bedrockvoicechat.com/wiki/reference/
---
Reference pages for the Bedrock Voice Chat server: every configuration key, environment variable, CLI command, and the HTTP API.

:::tip
For a first installation, use [Installing the BVC server](/wiki/server/installation/). It gives the same information in setup order.
:::

| Page | Contents |
|---|---|
| [config.hcl](/wiki/reference/configuration/) | Every configuration block and key, with defaults. |
| [Environment variables](/wiki/reference/environment-variables/) | The `BVC_*` overrides. Precedence: environment, then file, then default. |
| [CLI](/wiki/reference/cli/) | The server executable's command line: identities, users, permissions, bootstrapping. |
| [HTTP API](/wiki/reference/http-api/) | A link to the API reference, and the authentication schemes. Enable `openapi_docs` to browse the spec at `/docs`. |

These pages describe features that are still in development:

| Page | Contents |
|---|---|
| [Peering](/wiki/reference/peering/) | Voice links between servers. |
| [Assigned hostnames](/wiki/server/enrollment/) | A hostname and a certificate for a server with no domain. Available only to some Patreon and YouTube members. |