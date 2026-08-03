---
title: PaperMC
description: Install and configure the BVC plugin on a PaperMC server.
sidebar:
  label: PaperMC
  order: 3
---

## Install

1. Put the plugin JAR in your server's `plugins` directory.
2. Create the `plugins/BedrockVoiceChat` directory.
3. Create `plugins/BedrockVoiceChat/config.yaml`.
4. Start or restart the server.

## External mode

Points at a standalone [BVC server](/wiki/server/installation/). Use this unless you have a reason not to.

```yaml
bvc-server: "https://example.bedrockvc.stream"
access-token: "<MUST MATCH server.minecraft.access_token>"
minimum-players: 1
use-embedded-server: false
```

`access-token` has to match `server.minecraft.access_token` in the BVC server's `config.hcl`. A mismatch is rejected on every position update, which looks like the plugin silently not working.

## Embedded mode

Runs BVC inside the game server's JVM.

```yaml
bvc-server: "https://example.bedrockvc.stream"
access-token: "<GENERATE A SECURE TOKEN>"
minimum-players: 1
use-embedded-server: true

embedded:
  http-port: 8444
  quic-port: 8443
  broadcast-range: 32.0
  tls-certificate: "/etc/letsencrypt/live/example.bedrockvc.stream/fullchain.pem"
  tls-key: "/etc/letsencrypt/live/example.bedrockvc.stream/privkey.pem"
  tls-names:
    - example.bedrockvc.stream
  tls-ips:
    - 203.0.113.1
  log-level: info
  allow-audio-upload: false
  allow-audio-delete: false
```

The embedded block is `embedded:` here, against `embedded-config` in the Fabric JSON. Everything inside it is the same.

`bvc-server` is still what players type into the app, so set it to the public address even though nothing separate is running there.

The mTLS CA lands in `plugins/BedrockVoiceChat/`. Back it up.

## Then

[Whitelist your players](/wiki/server/players-and-permissions/) — BVC is deny-by-default — and send them to [Downloads](/wiki/start/downloads/).

Full key reference is on the [Java mod overview](/wiki/server/java-mod/).
