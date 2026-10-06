---
title: PaperMC
description: Install and configure the BVC plugin on a PaperMC server.
sidebar:
  label: PaperMC
  order: 3
---

The Bedrock Voice Chat PaperMC plugin reads its configuration from `plugins/BedrockVoiceChat/config.yml`.

## Install

1. Put the plugin JAR in the server's `plugins` directory.
2. Start the server once. The plugin writes `plugins/BedrockVoiceChat/config.yml`, with comments.
3. Stop the server, edit the file, and start the server again.

## External mode

The plugin sends player positions to a standalone [BVC server](/wiki/server/installation/).

```yaml
bvc-server: "https://bvc.example.com"
access-token: "<server.minecraft.access_token from config.hcl>"
```

Both keys are required. `access-token` must be the same as `server.minecraft.access_token` in the BVC server's `config.hcl`. If the values are different, the BVC server rejects each position update.

## Embedded mode

The plugin runs the BVC server inside the Minecraft server's JVM.

```yaml
use-embedded-server: true
embedded-config:
  server:
    port: 8444
    quic_port: 8443
    tls:
      names:
        - "bvc.example.com"
      acme:
        email: "you@example.com"
        provider: "cloudflare"
        api_token: "<Cloudflare API token>"
```

Replace `bvc.example.com` with your domain, and give it a Cloudflare API token scoped to that zone with DNS edit permission. BVC will automatically issue and renew the TLS certificate to secure voice communication between players. See [TLS](/wiki/server/tls/) to make the token, or to use acme-dns or your own certificate files.

`embedded-config` uses the keys of the BVC server's `config.hcl`, with the same nesting. A key that is not set uses the server default. See the [configuration reference](/wiki/reference/configuration/) for all keys.

Earlier Paper releases used the name `embedded:` for this block. The plugin does not read `embedded:` now. Use `embedded-config:`.

The plugin keeps its mTLS CA in `plugins/BedrockVoiceChat/`. Back up this directory. If you delete it, all client certificates that the server issued become invalid.

The [Java mod overview](/wiki/server/java-mod/#migration) has the full table of old keys and new paths.

## Chat sync

Chat sync needs this line in `server.properties`:

```properties
enforce-secure-profile=false
```

Restart the server after you change it. See [chat](/wiki/player/chat/).

## Next steps

1. [Add your players to the whitelist](/wiki/server/players-and-permissions/). BVC denies all players that are not on it.
2. Send your players to [Downloads](/wiki/start/downloads/).
