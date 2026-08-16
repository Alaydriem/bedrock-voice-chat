---
title: PaperMC
description: Install and configure the BVC plugin on a PaperMC server.
sidebar:
  label: PaperMC
  order: 3
---

The Bedrock Voice Chat PaperMC plugin runs on your Java server in external or embedded mode, configured through a YAML file.

## Install

1. Put the plugin JAR in your server's `plugins` directory.
2. Start the server once. The plugin writes a commented `plugins/BedrockVoiceChat/config.yml`.
3. Edit that file.
4. Restart the server.

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

`embedded-config` takes the same keys as the BVC server's own `config.hcl`, nested the same way. Anything you leave out takes the server's default.

```yaml
bvc-server: "https://example.bedrockvc.stream"
access-token: "<GENERATE A SECURE TOKEN>"
minimum-players: 1
use-embedded-server: true

embedded-config:
  server:
    port: 8444
    quic_port: 8443
    tls:
      certificate: "/etc/letsencrypt/live/example.bedrockvc.stream/fullchain.pem"
      key: "/etc/letsencrypt/live/example.bedrockvc.stream/privkey.pem"
      names:
        - example.bedrockvc.stream
      ips:
        - 203.0.113.1
    bedrock:
      enabled: true
  voice:
    spatial_audio:
      broadcast_range: 32.0
  log:
    level: info
  permissions:
    defaults:
      audio_upload: false
      audio_delete: false
```

The block is `embedded-config` on both Paper and Fabric. Earlier Paper releases called it `embedded:`. That name no longer works.

Set `server.port` and `server.quic_port` explicitly. Both default to 443. Binding 443 needs root on Linux.

`bvc-server` is still what players type into the app. Set it to the public address even though nothing separate runs there.

The mTLS CA lands in `plugins/BedrockVoiceChat/`. Back it up.

## Migrating from the old flat keys

If your embedded block still has `http-port`, `quic-port`, `tls-certificate` and the rest, the plugin will refuse to start and log each key with its replacement. Before:

```yaml
use-embedded-server: true
embedded:
  http-port: 8444
  quic-port: 8443
  broadcast-range: 32.0
  tls-certificate: "/etc/bvc/fullchain.pem"
  tls-key: "/etc/bvc/privkey.pem"
  tls-names:
    - bvc.example.com
  log-level: info
```

After:

```yaml
use-embedded-server: true
embedded-config:
  server:
    port: 8444
    quic_port: 8443
    tls:
      certificate: "/etc/bvc/fullchain.pem"
      key: "/etc/bvc/privkey.pem"
      names:
        - bvc.example.com
  voice:
    spatial_audio:
      broadcast_range: 32.0
  log:
    level: info
```

The full mapping table is on the [Java mod overview](/wiki/server/java-mod/#migration).

## Chat sync

Set this in `server.properties`:

```properties
enforce-secure-profile=false
```

Chat sync does not work with it enabled. Restart the server after changing it.

Everything else is automatic in both external and embedded mode. See [chat](/wiki/player/chat/).

## Then

[Whitelist your players](/wiki/server/players-and-permissions/). BVC is deny-by-default. Then send them to [Downloads](/wiki/start/downloads/).

Full key reference is on the [Java mod overview](/wiki/server/java-mod/).
