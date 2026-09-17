---
title: "HTTP API"
description: "Where the BVC server's API reference lives, and how it is organised."
source: https://www.bedrockvoicechat.com/wiki/reference/http-api/
---
Bedrock Voice Chat server exposes an HTTP API for authentication, configuration, position relay, and administration.

## Reference

**[bedrockvoicechat.com/api](https://www.bedrockvoicechat.com/api)**

## Serving the spec from your own server

Your server can serve the spec for its own build:

```hcl
server {
    features {
        openapi_docs = true
    }
}
```

- `/openapi.json` — the spec
- `/docs` — browsable UI

## Authentication

| Scheme | Used by |
|---|---|
| **mTLS** | Player and admin endpoints. The client certificate is issued at login. |
| **`Authorization: Bearer`** | Game server endpoints — the position relay. The shared secret from `server.minecraft.access_token`. |
| **None** | Health, config, and the login flow itself. |