---
title: HTTP API
description: Where the BVC server's API reference lives, and how it is organised.
sidebar:
  label: HTTP API
  order: 5
---

## Reference

**[bedrockvoicechat.com/api](https://www.bedrockvoicechat.com/api)**

Generated from the server source on every build and published as a browsable reference, so it always matches the released server. Paths, request and response schemas, and status codes live there.

Builds off `master` publish to `/api`; other branches publish to `/api/beta`.

This page does not restate any of it. A hand-maintained endpoint list would drift from the generated one, and there would be no way to tell which was wrong.

## Serving the spec from your own server

Your server can serve its own spec, matching your exact build rather than the released one:

```hcl
server {
    features {
        openapi_docs = true
    }
}
```

- `/openapi.json` — the spec
- `/docs` — browsable UI

Off by default. Worth turning on temporarily when you are building against a version that is not the current release, or debugging a schema mismatch. The published reference is the better default for everything else.

## Authentication

Three schemes, which is the thing worth knowing before you open the reference:

| Scheme | Used by |
|---|---|
| **mTLS** | Player and admin endpoints. The client certificate is issued at login. |
| **`X-MC-Access-Token`** | Game server endpoints — position relay and Bedrock transfer. The shared secret from `server.minecraft.access_token`. |
| **None** | Health, config, and the login flow itself. |

## Route groups

| Prefix | Covers |
|---|---|
| `/api/auth/*` | Sign-in, code redemption, introspection, Java account linking |
| `/api/position` | Position relay from the game server |
| `/api/channel/*` | Channels — surfaced in the app as [Groups](/wiki/player/groups/) |
| `/api/audio/*` | The [audio library](/wiki/player/audio-library/) |
| `/api/control`, `/api/state`, `/api/preferences` | In-game control surface |
| `/api/admin/*` | User and permission management, driven by the [CLI](/wiki/reference/cli/) |
| `/api/relay/*` | [Cross-server peering](/wiki/reference/peering/). Early access |
| `/api/bedrock/transfer` | Bedrock relay handoff |
| `/health/*`, `/api/ping` | Probes. See [monitoring](/wiki/server/monitoring/) |
| `/metrics` | Prometheus metrics |

## Not in the spec

Three routes are mounted but absent from the generated reference, because their bodies cannot be described as JSON schemas:

| Route | Why |
|---|---|
| `POST /api/audio/file` | Raw multipart body |
| `GET /api/audio/stream` | Byte stream response |
| `GET /api/websocket/positions` | WebSocket upgrade, not a JSON response |

They work; they are simply not describable in OpenAPI. Upload takes the file as the request body with the filename in a header; stream takes a token minted by `POST /api/audio/file/{id}/token`.
