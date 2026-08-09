---
title: Monitoring
description: Prometheus metrics, health endpoints, and logging.
sidebar:
  label: Monitoring
  order: 10
---

## Health endpoints

| Endpoint | Answers |
|---|---|
| `/health/liveness` | Is the process up? Restart on failure. |
| `/health/readiness` | Can it take traffic? Gate the load balancer on this. |
| `/api/ping` | Cheap reachability probe. |

None require authentication. They work as container health checks without credentials. See [Docker](/wiki/server/docker/) for probe configuration.

## Metrics

`/metrics` serves Prometheus format. It is always on, independent of the `telemetry` feature flag. That flag controls what BVC sends to us, not what you can scrape.

```yaml
scrape_configs:
  - job_name: bvc
    scheme: https
    static_configs:
      - targets: ['example.bedrockvc.stream:443']
```

## Logging

```hcl
log {
    level = "info"
    out   = "stdout"
}
```

`trace`, `debug`, `info`, `warn`, `error`. Default `info`.

Use `debug` when connections establish and then fail. It logs the QUIC path and handshake detail. It is loud. Do not leave it on.

## What to watch

**Voice failing while HTTP works.** This is the characteristic BVC failure. The two use different transports on the same port. A player signs in fine and hears nothing. Alert on QUIC connection counts. An HTTP-only check reports green through the entire outage.

**UDP reachability.** Most broken deployments have UDP/443 closed. Use an external probe. Do not trust the firewall config.

**Certificate expiry**, if you supply your own certificates. ACME renews itself. Manual PEM files do not.

**Disk**, if `audio_upload` is broadly granted. The audio library has no per-player quota. Client-side recordings are separate and also unbounded.

## Confirming a deployment from outside

```bash
curl https://example.bedrockvc.stream/api/config
```

JSON means HTTP, TLS, and the port are fine. It does not prove voice works. QUIC over UDP is a separate path and the most common thing to be broken. Test with a real client.
