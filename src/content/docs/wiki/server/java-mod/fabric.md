---
title: Fabric
description: Install and configure the BVC mod on a Fabric server.
sidebar:
  label: Fabric
  order: 2
---

The Bedrock Voice Chat Fabric mod runs on your Java server in external or embedded mode, configured through a JSON file.

## Install

1. Put [fabric-api](https://modrinth.com/mod/fabric-api) in your server's `mods` folder.
2. Put `bedrock-voice-chat.jar` in `mods` as well.
3. Create `config/bedrock-voice-chat.json`.
4. Start the server.

## External mode

Points at a standalone [BVC server](/wiki/server/installation/). Use this unless you have a reason not to.

```json
{
  "bvc-server": "https://example.bedrockvc.stream",
  "access-token": "<MUST MATCH server.minecraft.access_token>",
  "minimum-players": 1,
  "use-embedded-server": false
}
```

`access-token` has to match `server.minecraft.access_token` in the BVC server's `config.hcl`. A mismatch is rejected on every position update, which looks like the mod silently not working.

## Embedded mode

Runs BVC inside the game server's JVM.

`embedded-config` takes the same keys as the BVC server's own `config.hcl`, nested the same way. Anything you leave out takes the server's default.

```json
{
  "bvc-server": "https://example.bedrockvc.stream",
  "access-token": "<GENERATE A SECURE TOKEN>",
  "minimum-players": 1,
  "use-embedded-server": true,
  "embedded-config": {
    "server": {
      "port": 8444,
      "quic_port": 8443,
      "tls": {
        "certificate": "/etc/letsencrypt/live/example.bedrockvc.stream/fullchain.pem",
        "key": "/etc/letsencrypt/live/example.bedrockvc.stream/privkey.pem",
        "names": ["example.bedrockvc.stream"],
        "ips": ["203.0.113.1"]
      },
      "bedrock": {
        "enabled": true
      }
    },
    "voice": {
      "spatial_audio": { "broadcast_range": 32.0 }
    },
    "log": { "level": "info" },
    "permissions": {
      "defaults": { "audio_upload": false, "audio_delete": false }
    }
  }
}
```

Set `server.port` and `server.quic_port` explicitly. Both default to 443. Binding 443 needs root on Linux.

`bvc-server` is still what players type into the app. Set it to the public address even though nothing separate runs there.

On Windows, use forward slashes or doubled backslashes in paths: `C:/certs/fullchain.pem` or `C:\\certs\\fullchain.pem`.

The mTLS CA lands in `config/bedrock-voice-chat/`. Back it up.

## Migrating from the old flat keys

If your `embedded-config` still has `http-port`, `quic-port`, `tls-certificate` and the rest, the mod will refuse to start and log each key with its replacement. Before:

```json
{
  "use-embedded-server": true,
  "embedded-config": {
    "http-port": 8444,
    "quic-port": 8443,
    "broadcast-range": 32.0,
    "tls-certificate": "/etc/bvc/fullchain.pem",
    "tls-key": "/etc/bvc/privkey.pem",
    "tls-names": ["bvc.example.com"],
    "log-level": "info"
  }
}
```

After:

```json
{
  "use-embedded-server": true,
  "embedded-config": {
    "server": {
      "port": 8444,
      "quic_port": 8443,
      "tls": {
        "certificate": "/etc/bvc/fullchain.pem",
        "key": "/etc/bvc/privkey.pem",
        "names": ["bvc.example.com"]
      }
    },
    "voice": { "spatial_audio": { "broadcast_range": 32.0 } },
    "log": { "level": "info" }
  }
}
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
