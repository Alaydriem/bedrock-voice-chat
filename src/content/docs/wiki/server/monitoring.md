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

None require authentication, so they work as container health checks without credentials. See [Docker](/wiki/server/docker/) for probe configuration.

## Metrics

`/metrics` serves Prometheus format. It is always on, independent of the `telemetry` feature flag — that flag controls what BVC sends to us, not what you can scrape.

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

`debug` is worth reaching for when connections establish and then fail — it logs the QUIC path and handshake detail that explains why. It is loud, so do not leave it on.

## What to actually watch

**Voice failing while HTTP works** is the characteristic BVC failure, because the two use different transports on the same port. A player signs in fine and hears nothing. Alert on QUIC connection counts, not just HTTP availability — an HTTP-only check will report green through the entire outage.

**UDP reachability.** Most deployments that break do so because UDP/443 is not actually open. It is worth an external probe rather than trusting the firewall config.

**Certificate expiry**, if you supply your own certificates. ACME handles renewal itself; manual PEM files do not.

**Disk**, if `audio_upload` is broadly granted. There is no per-player quota on the audio library, and recordings on the client side are separate but also unbounded.

## Confirming a deployment from outside

```bash
curl https://example.bedrockvc.stream/api/config
```

JSON means HTTP, TLS, and the port are all fine. This does not prove voice works — QUIC over UDP is a separate path and the most common thing to be broken. Test with an actual client before declaring victory.
