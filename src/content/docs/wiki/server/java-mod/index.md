---
title: Java mod
description: BVC on Fabric and PaperMC, in external or embedded mode.
sidebar:
  label: Overview
  order: 1
---

BVC runs on Java servers through a mod published on [Modrinth](https://modrinth.com/mod/bedrock-voice-chat) for both Fabric and PaperMC. Geyser servers work. See [Geyser](/wiki/platforms/geyser/).

## Pick your loader

- **[Fabric](/wiki/server/java-mod/fabric/)** — `mods/` directory, JSON config.
- **[PaperMC](/wiki/server/java-mod/papermc/)** — `plugins/` directory, YAML config.

The settings are identical. Only the file format and location differ.

## Pick your mode

**External** points at a standalone BVC server. Use it for production, for more than about 15 players, or when one BVC server serves several game servers.

**Embedded** runs BVC inside the game server's JVM. No separate service. Suits small servers and local development.

Embedded needs an always-on host with adequate RAM. Low-end hosts, and anything that suspends when idle, produce audio quality and performance problems that are hard to attribute. Move BVC to its own machine if you hit them.

## Shared settings

| Key | Default | Description |
|---|---|---|
| `bvc-server` | — | URL of the BVC server. Required in external mode. |
| `access-token` | — | Must match `server.minecraft.access_token` on the BVC server. Required in external mode. |
| `minimum-players` | `1` | Players online before positions are relayed. |
| `use-embedded-server` | `false` | Run BVC inside this JVM. |
| `embedded-config` | — | Required when `use-embedded-server` is true. |

Keys accept both `hyphen-case` and `camelCase`.

## Embedded configuration

:::caution[This changed]
`embedded-config` used to take a short list of flat keys of its own. Those keys are gone.
The mod refuses to start in embedded mode if it finds them. See [Migration](#migration).
:::

`embedded-config` takes the **same keys as the BVC server's own `config.hcl`**, in the same nesting, under the same names. Anything you leave out takes the server's default.

The Kotlin classes are generated from the server's Rust configuration types. A key the server accepts is a key the mod accepts. Use the [configuration reference](/wiki/reference/configuration/) as the key list for this block.

Four sections are unavailable: `server.meridian`, `server.cors`, `server.features.openapi_docs`, and `server.bedrock.servers`.

### Defaults to set explicitly

Four defaults come from the standalone server and are wrong for most Java hosts.

| Key | Default | Set it to |
|---|---|---|
| `server.port` | `443` | `8444`. Binding 443 needs root on Linux, and a failed HTTP bind stops startup. |
| `server.quic_port` | `443` | `8443`. |
| `server.bedrock.transfer_port` | `19132` | `19139`. Geyser normally owns 19132. A failed bind is logged and the server keeps running, and the relay then appears to do nothing. |
| `voice.spatial_audio.broadcast_range` | `48.0` | Whatever range you want, in blocks. |

Set `server.bedrock.enabled: false` to turn the relay off instead.

`server.bedrock.dns.enabled` stays false. Port 53 is untouched unless you ask for it.

### TLS

Set `server.tls.certificate` and `server.tls.key`, **or** a `server.tls.acme` block. The mod refuses to start with neither. The server refuses both at once.

Embedded mode supports ACME DNS-01:

```yaml
embedded-config:
  server:
    tls:
      names:
        - "bvc.example.com"
      acme:
        email: "ops@example.com"
        provider: "cloudflare"
        api_token: "..."
```

Providers: `cloudflare` and `acme-dns`. See [TLS](/wiki/server/tls/).

### Environment overrides

`BVC_*` variables apply to an embedded server and outrank the config file. Precedence is **environment > config file > default**. A malformed value stops startup and names the variable.

See [environment variables](/wiki/reference/environment-variables/). `BVC_SERVER`, `BVC_BEDROCK_SERVERS`, and the `BVC_MERIDIAN_*` set do not apply here.

## Migration

The flat keys no longer work. The mod refuses to start in embedded mode and logs each one it found alongside its replacement path.

| Old key | New path |
|---|---|
| `http-port` | `server.port` |
| `quic-port` | `server.quic_port` |
| `broadcast-range` | `voice.spatial_audio.broadcast_range` |
| `tls-certificate` | `server.tls.certificate` |
| `tls-key` | `server.tls.key` |
| `tls-names` | `server.tls.names` |
| `tls-ips` | `server.tls.ips` |
| `log-level` | `log.level` |
| `assets-path` | `server.assets_path` |
| `allow-audio-upload` | `permissions.defaults.audio_upload` |
| `allow-audio-delete` | `permissions.defaults.audio_delete` |

Expect one behaviour change. **`broadcast-range` never worked.** The mod sent it at a path no server field matched. It was discarded, and every embedded server ran at 48 whatever you set. Under `voice.spatial_audio.broadcast_range` it now applies, and a config that set 32 will sound different.

Per-format before and after examples are on the [Fabric](/wiki/server/java-mod/fabric/) and [PaperMC](/wiki/server/java-mod/papermc/) pages.

## Chat sync

Set `enforce-secure-profile=false` in `server.properties`. Chat sync does not work with it enabled.

Everything else is automatic in both modes. See [chat](/wiki/player/chat/).

## Certificates in embedded mode

Two systems. One is yours.

| | Purpose | Managed by |
|---|---|---|
| HTTPS TLS | Clients trusting the server | You. `server.tls.certificate` and `server.tls.key` from a public CA, or `server.tls.acme` |
| QUIC mTLS CA | Server authenticating clients | The mod, automatically |

The mTLS CA is generated into the mod's data directory: `config/bedrock-voice-chat/` on Fabric, `plugins/BedrockVoiceChat/` on Paper. Leave it alone and back it up. Deleting it invalidates every client certificate already issued.

## Support

Issues with the mod go to [GitHub](https://github.com/Alaydriem/bedrock-voice-chat/issues).
