---
title: Fabric
description: Install and configure the BVC mod on a Fabric server.
sidebar:
  label: Fabric
  order: 2
---

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

```json
{
  "bvc-server": "https://example.bedrockvc.stream",
  "access-token": "<GENERATE A SECURE TOKEN>",
  "minimum-players": 1,
  "use-embedded-server": true,
  "embedded-config": {
    "http-port": 8444,
    "quic-port": 8443,
    "broadcast-range": 32.0,
    "tls-certificate": "/etc/letsencrypt/live/example.bedrockvc.stream/fullchain.pem",
    "tls-key": "/etc/letsencrypt/live/example.bedrockvc.stream/privkey.pem",
    "tls-names": ["example.bedrockvc.stream"],
    "tls-ips": ["203.0.113.1"],
    "log-level": "info",
    "allow-audio-upload": false,
    "allow-audio-delete": false
  }
}
```

`bvc-server` is still what players type into the app, so set it to the public address even though nothing separate is running there.

On Windows, use forward slashes or doubled backslashes in paths: `C:/certs/fullchain.pem` or `C:\\certs\\fullchain.pem`.

The mTLS CA lands in `config/bedrock-voice-chat/`. Back it up.

## Then

[Whitelist your players](/wiki/server/players-and-permissions/) — BVC is deny-by-default — and send them to [Downloads](/wiki/start/downloads/).

Full key reference is on the [Java mod overview](/wiki/server/java-mod/).
