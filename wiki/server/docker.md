---
title: "Docker and Kubernetes"
description: "Compose, volumes, health checks, and running BVC entirely from environment variables."
source: https://www.bedrockvoicechat.com/wiki/server/docker/
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
      - "28280:28280/udp"
      - "28284:28284/udp"
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

The certificates directory (`server.tls.certs_path`) can be temporary. At startup, the server writes the CA into it from the database. If the directory is empty, the server fills it again. Set `server.tls.certs_path` to a temporary directory if a persistent volume is not practical.

:::caution
Your server's own TLS certificate is different from the mTLS CA. The server reads `server.tls.certificate` and `server.tls.key` from the path where you mount them. They are not stored in the database.
:::

## Running without a config file

Most settings have a `BVC_*` environment variable. You do not need a `config.hcl` file. When there is no `config.hcl`, the server starts with the defaults and the environment variables, and writes a log line that says so.

On Kubernetes, configure BVC with environment variables. Put secret values in Kubernetes Secrets. You do not need a ConfigMap. See [environment variables](/wiki/reference/environment-variables/).

A variable that is not set, or that is empty, does not change a value. `FOO=` in a compose file does not clear a configured value. If a variable has an invalid value, the server does not start.

## Health checks

| Endpoint | Use |
|---|---|
| `/health/liveness` | The process is running. Restart the container if this fails. |
| `/health/readiness` | The server can accept traffic. Use it for the load balancer check. |
| `/api/ping` | A light check that the server is reachable. |

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

The server handles SIGTERM. `docker stop` and pod eviction close connections correctly.

## Scaling

BVC is limited by CPU, and each connection keeps state on one instance. Add CPU cores to one instance before you add instances.

Do not put several instances behind an ordinary load balancer. A QUIC connection stays open for a long time and must stay on the instance that opened it. Round-robin routing breaks voice. To share one address between instances, the load balancer must route by QUIC connection ID.