---
title: Reference
description: Configuration, environment variables, CLI, and the HTTP API.
sidebar:
  label: Overview
  order: 1
---

Lookup material for the Bedrock Voice Chat server: every configuration key, environment override, CLI command, and HTTP endpoint.

:::tip
Setting up for the first time? Use [Installing the BVC server](/wiki/server/installation/) instead. It covers the same ground in the order you need it.
:::

| Page | What it covers |
|---|---|
| [config.hcl](/wiki/reference/configuration/) | Every configuration block and key, with defaults. |
| [Environment variables](/wiki/reference/environment-variables/) | Every `BVC_*` override. Precedence is environment > file > default. |
| [CLI](/wiki/reference/cli/) | The server executable's command line: identities, users, permissions, bootstrapping. |
| [HTTP API](/wiki/reference/http-api/) | Endpoint index. Enable `openapi_docs` to browse the live spec at `/docs`. |

Under **Work in progress**:

| Page | What it covers |
|---|---|
| [Peering](/wiki/reference/peering/) | Cross-server voice links. |
| [Peer relay](/wiki/server/peer-relay/) | The iroh relay peers connect through when no direct path exists. |
