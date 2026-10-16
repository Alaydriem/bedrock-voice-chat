---
title: "PaperMC"
description: "Install and configure the BVC plugin on a PaperMC server."
source: https://www.bedrockvoicechat.com/wiki/server/java-mod/papermc/
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

Replace `bvc.example.com` with your domain. Set `api_token` to a Cloudflare API token with DNS edit permission on that zone. BVC gets and renews the TLS certificate automatically. See [TLS](/wiki/server/tls/) to make the token, or to use acme-dns or your own certificate files.

`embedded-config` uses the same keys and the same nesting as the BVC server's `config.hcl`. A key that is not set uses the server default. See the [configuration reference](/wiki/reference/configuration/) for all keys.

The plugin keeps its certificate authority (the key that signs player certificates) in `plugins/BedrockVoiceChat/`. Back up this directory. If you delete it, all client certificates that the server issued become invalid.

:::caution
Earlier Paper releases used `embedded:` for this block. The plugin ignores `embedded:`. Rename it to `embedded-config:`.

Releases before 1.0.0-beta.21 used flat keys in this block. [Upgrading to beta.21](/wiki/upgrading/beta-21/#java-mod) has the table of old keys and new keys.
:::

## Chat sync

To read and send in-game [chat](/wiki/player/chat/) from the BVC app, set this line in `server.properties`:

```properties
enforce-secure-profile=false
```

Restart the server after you change it.

## Next steps

1. Let your players in. A player can sign in after they join your game server once, or after you [add them](/wiki/server/players-and-permissions/).
2. Send your players to [Downloads](/wiki/start/downloads/).