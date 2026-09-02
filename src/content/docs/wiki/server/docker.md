---
title: Docker and Kubernetes
description: Compose, volumes, health checks, and running BVC entirely from environment variables.
sidebar:
  label: Docker & K8
  order: 5
---

Bedrock Voice Chat server ships as a container image at `ghcr.io/alaydriem/bedrock-voice-chat/server`, built for amd64 and arm64.

## Compose

```yaml
services:
  bedrock-voice-chat:
    image: ghcr.io/alaydriem/bedrock-voice-chat/server:<VERSION>
    restart: always
    ports:
      - "443:443/tcp"
      - "443:443/udp"
      - "28280:28280/udp"
      - "28283:28283/udp"
    user: "100:101"
    cap_add:
      - NET_BIND_SERVICE
    volumes:
      - ./:/data
    healthcheck:
      test: ["CMD", "curl", "-fsS", "https://localhost/health/readiness"]
      interval: 30s
      timeout: 5s
      retries: 3
      start_period: 30s
```

## What to persist

Mount `/data` and keep it. It holds:

- `config.hcl`
- The SQLite database, if you are not using MySQL or Postgres

The database holds the mTLS certificate authority. Losing it invalidates every client certificate already issued and forces everyone to sign in again.

`certificates/` does not have to survive a restart. The server writes the CA there at startup from the database, and repopulates an empty directory. Point `server.tls.certs_path` at a temporary directory where a persistent volume is inconvenient.

:::caution
Serving certificates are separate. A certificate and key given through `server.tls.certificate` and `server.tls.key` are read from the path you mount them at, and are not stored in the database.
:::

## Running without a config file

Every setting has a `BVC_*` override, which makes the file optional. With no `config.hcl` the server starts from defaults plus the environment and logs that it did.

That is usually the right approach for Kubernetes: no ConfigMap for the file, secrets as secrets, everything else as env. See [environment variables](/wiki/reference/environment-variables/).

An unset or empty variable never overrides anything. `FOO=` in a compose file will not blank out a configured value. A malformed one is a hard startup error.

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

The server handles SIGTERM. `docker stop` and pod eviction close connections cleanly.

## Scaling

BVC is CPU-bound and stateful per connection. Scale a single instance up with more cores before reaching for more replicas.

Do not put several instances behind an ordinary load balancer. QUIC connections are long-lived and must stay pinned to the instance that established them. Round-robin breaks voice. Sharing one address across instances needs routing by connection ID, which our managed hosting handles.
