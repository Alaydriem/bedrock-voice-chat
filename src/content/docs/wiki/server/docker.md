---
title: Docker and Kubernetes
description: Compose, volumes, health checks, and running BVC entirely from environment variables.
sidebar:
  label: Docker
  order: 4
---

The container image is `ghcr.io/alaydriem/bedrock-voice-chat/server`, built for amd64 and arm64.

## Compose

```yaml
services:
  bvc:
    image: ghcr.io/alaydriem/bedrock-voice-chat/server:<version>
    restart: always
    ports:
      - "443:443/tcp"
      - "443:443/udp"
      - "8443:8443/udp"
    volumes:
      - ./data:/data
    environment:
      BVC_ACCESS_TOKEN: ${BVC_ACCESS_TOKEN}
      BVC_ACME_EMAIL: you@example.com
      BVC_ACME_PROVIDER: cloudflare
      BVC_ACME_API_TOKEN: ${CF_API_TOKEN}
      BVC_TLS_NAMES: example.bedrockvc.stream
    healthcheck:
      test: ["CMD", "curl", "-fsS", "https://localhost/health/readiness"]
      interval: 30s
      timeout: 5s
      retries: 3
      start_period: 30s
```

## Ports

| Port | Protocol | Carries |
|---|---|---|
| 443 | TCP | HTTP API, login |
| 443 | UDP | QUIC voice |
| 8443 | UDP | Redundant QUIC, for networks that block it on 443 |

Publishing TCP only produces a server that authenticates fine and never passes audio. It is the most common container misconfiguration because everything looks healthy until someone tries to talk.

Add 53/UDP if you enable the DNS responder for console players — see [console and mobile](/wiki/platforms/console-and-mobile/). Binding 53 inside a container needs `NET_BIND_SERVICE`.

## What to persist

Mount `/data` and keep it. It holds:

- `config.hcl`
- The SQLite database, if you are not using MySQL or Postgres
- `certificates/` — the mTLS CA

Losing the mTLS CA invalidates every client certificate already issued and forces everyone to sign in again. Back it up with the database, not separately.

## Running without a config file

Every setting has a `BVC_*` override, so the file is optional. With no `config.hcl` the server starts from defaults plus the environment and logs that it did.

That is usually the right approach for Kubernetes: no ConfigMap for the file, secrets as secrets, everything else as env. See [environment variables](/wiki/reference/environment-variables/).

An unset or empty variable never overrides anything, so `FOO=` in a compose file will not blank out a configured value. A malformed one is a hard startup error rather than a silent fallback.

## Health checks

| Endpoint | Use |
|---|---|
| `/health/liveness` | Process is up. Restart on failure. |
| `/health/readiness` | Ready for traffic. Gate the load balancer on it. |
| `/api/ping` | Cheap reachability probe. |

Kubernetes:

```yaml
livenessProbe:
  httpGet: { path: /health/liveness, port: 443, scheme: HTTPS }
  initialDelaySeconds: 30
readinessProbe:
  httpGet: { path: /health/readiness, port: 443, scheme: HTTPS }
  periodSeconds: 10
```

## Graceful shutdown

The server handles SIGTERM, so `docker stop` and pod eviction close connections cleanly rather than dropping them.

## Scaling

BVC is CPU-bound and stateful per connection. Scale a single instance up with more cores before reaching for more replicas.

Do not put several instances behind an ordinary load balancer. QUIC connections are long-lived and must stay pinned to the instance that established them, so round-robin breaks voice. Sharing one address across instances needs routing by connection ID, which is what our managed hosting handles.
