---
title: "Monitoring"
description: "Prometheus metrics, health endpoints, and logging."
source: https://www.bedrockvoicechat.com/wiki/server/monitoring/
---
Bedrock Voice Chat server has health endpoints, Prometheus metrics, and structured logs for monitoring a deployment.

## Health endpoints

| Endpoint | Use |
|---|---|
| `/health/liveness` | The process is running. Restart the container if this fails. |
| `/health/readiness` | The server can accept traffic. Use it for the load balancer check. |
| `/api/ping` | A light check that the server is reachable. |

These endpoints need no authentication. See [Docker](/wiki/server/docker/) for probe configuration.

## Metrics

`/metrics` serves Prometheus format. It is always on. The `telemetry` feature flag controls only the data that BVC sends to the BVC developers. It does not change `/metrics`.

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
    path  = "./logs"
}
```

`level` is `trace`, `debug`, `info`, `warn`, or `error`. The default is `info`.

Use `debug` when connections open and then fail. At `debug`, the server logs QUIC path and handshake details. It writes many log lines. Set the level back to `info` when you are done.

The server always writes to both outputs:

- **stderr**: human-readable text with colour. Colour turns off when stderr is not a terminal.
- **`log.path`**: one JSON object per line. The file rotates at 50 MB, and the server keeps ten old files.

Send the JSON log file to your log system. Do not use the console output. Every line has `ts`, `level`, `target`, `msg`, `instance_id`, `name`, and `boot_id`. Some lines have more fields, as top-level keys.

```bash
jq -r 'select(.level=="warn" or .level=="error") | "\(.ts) \(.target) \(.msg)"' logs/bvc-server.log
```

`boot_id` changes each time the server starts. Use it to find the lines from one run in a file.

`RUST_LOG` overrides `level` and the default level of each module. It applies to the server and to its libraries:

```bash
RUST_LOG=info,bvc_server_lib::stream=debug,hyper=off
```

## What to watch

**Voice fails while HTTP works.** HTTP uses TCP/443. Voice uses QUIC on UDP/28280. If UDP is blocked, players can sign in but hear nothing, and HTTP checks still pass. Alert on the QUIC connection count.

**UDP reachability.** In most broken deployments, the UDP port is closed. Test it with a probe from outside your network. Do not rely on the firewall configuration.

**Certificate expiry**, if you supply your own certificates. BVC renews ACME certificates. It does not renew PEM files that you supply.

**Disk space**, if many players have `audio_upload`. The audio library has no limit for each player.

## Checking a deployment from outside

```bash
curl https://example.bedrockvc.stream/api/config
```

A JSON response shows that HTTP, TLS and TCP/443 work. It does not show that voice works. Voice uses QUIC over UDP, and a closed UDP port is the most common fault. Test voice with a real client.