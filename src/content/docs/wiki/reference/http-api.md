---
title: HTTP API
description: Location of the BVC server API reference, and its authentication schemes.
sidebar:
  label: HTTP API
  order: 5
---

The Bedrock Voice Chat server has an HTTP API for authentication, configuration, player positions, and administration.

## Reference

The full API reference is at [bedrockvoicechat.com/api](https://www.bedrockvoicechat.com/api).

## Serving the spec from your own server

Your server can serve the spec for its own build:

```hcl
server {
    features {
        openapi_docs = true
    }
}
```

| Path | Content |
|---|---|
| `/openapi.json` | The OpenAPI spec. |
| `/docs` | A browsable UI for the spec. |

## Authentication

| Scheme | Used by |
|---|---|
| **mTLS** | Player and admin endpoints. The server issues the client certificate at login. |
| **`Authorization: Bearer`** | Endpoints that the Addon or the Java mod calls to send player positions. The token is the value of `server.minecraft.access_token`. |
| **None** | Health, config, and the login endpoints. |
