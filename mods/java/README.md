# Bedrock Voice Chat — Java mods

Fabric, Paper and PowerNukkitX builds of the BVC mod. All three share `:common`, and
all three read the same configuration shape.

- Fabric: `config/bedrock-voice-chat.json`
- Paper: `plugins/BedrockVoiceChat/config.yml`
- PowerNukkitX: `plugins/BedrockVoiceChat/config.yml`

## External or embedded

`use-embedded-server` decides which half of the configuration matters.

**External** — the mod talks to a BVC server someone else runs. Requires
`bvc-server` and `access-token`. The remote operator owns that server's
configuration; nothing under `embedded-config` applies.

```yaml
bvc-server: "https://bvc.example.com"
access-token: "..."
minimum-players: 1
use-embedded-server: false
```

**Embedded** — the mod runs a BVC server inside the Minecraft server process.
`embedded-config` configures it, and `bvc-server` is ignored.

```yaml
use-embedded-server: true
embedded-config:
  server:
    port: 8444
    quic_port: 8443
    tls:
      play: {}
```

## The embedded configuration mirrors the server

`embedded-config` has the same keys as the BVC server's own configuration file,
in the same nesting, with the same names. Anything left out takes the server's
default rather than a mod-specific one.

The Kotlin classes for it are **generated from the Rust config types**, so a key
the server accepts is a key the mod accepts.

Four sections are deliberately unavailable, because they do not apply to a
self-hosted game server: `server.meridian`, `server.cors`,
`server.features.openapi_docs`, and `server.bedrock.servers`.

### Defaults worth setting explicitly

**Ports.** `server.port` and `server.quic_port` both default to **443**. On
Linux, binding 443 needs root or `CAP_NET_BIND_SERVICE`, and the HTTP listener's
bind failure stops startup. Set 8444 and 8443 unless you intend 443.

### TLS is required

Use exactly one certificate source, in the file or through its `BVC_*`
variables (see below). The mod refuses to start with none, and the server
refuses two.

| Source | Key | Needs a domain |
| --- | --- | --- |
| Play name | `server.tls.play` | No |
| Enrollment | `server.enrollment.token` | No |
| ACME | `server.tls.acme` | Yes |
| Your own certificate | `server.tls.certificate` and `server.tls.key` | Yes |

**Play name.** The BVC registry assigns the server a `*.play.bedrockvc.stream`
name and a 15-day certificate, and renews it. Nothing else is needed:

```yaml
embedded-config:
  server:
    tls:
      play: {}
```

Write the empty braces. In YAML, `play:` with nothing after it is null, and a
null block leaves play mode off. In Fabric's JSON, write `"play": {}`.

`play.address` is optional. It is the public IP players reach, and the registry
publishes it as the name's DNS record. Unset, the registry uses the address it
sees the server connect from. It must be an IP address, not a hostname, and the
registry refuses a private address.

```yaml
      play:
        address: "203.0.113.10"
```

**Enrollment.** For entitled members. The registry assigns your reserved name
and its certificate. The token is spent on the first start; after that, the
server's node key authenticates it, so the token can stay or be removed.
`address` and `port` are optional, with the same meaning as for play.

```yaml
embedded-config:
  server:
    enrollment:
      token: "..."
```

**ACME.** Obtains a certificate for your own domain over DNS-01:

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

Supported providers: `cloudflare`, `acme-dns`.

## Environment overrides

`BVC_*` variables apply to an embedded server and take precedence over the
configuration file: env > config file > default. A malformed value stops startup
and names the variable.

`BVC_ACCESS_TOKEN`, `BVC_QUIC_PORT`, `BVC_ADVERTISED_QUIC_PORTS`,
`BVC_TELEMETRY`, `BVC_CHAT`, `BVC_TLS_CERTIFICATE`, `BVC_TLS_KEY`, `BVC_TLS_CERTS_PATH`,
`BVC_TLS_NAMES`, `BVC_TLS_IPS`, `BVC_ACME_EMAIL`, `BVC_ACME_PROVIDER`,
`BVC_ACME_API_TOKEN`, `BVC_ACME_DIRECTORY`, `BVC_ACME_DOMAINS`,
`BVC_ACME_DNS_URL`, `BVC_ACME_DNS_USERNAME`, `BVC_ACME_DNS_PASSWORD`,
`BVC_ACME_DNS_SUBDOMAIN`, `BVC_ENROLLMENT_TOKEN`, `BVC_ENROLLMENT_ADDRESS`,
`BVC_PLAY` (`true` adds a play block, `false` removes one), `BVC_PLAY_ADDRESS`
(needs play mode on),
`BVC_DATABASE_SCHEME`, `BVC_DATABASE_DATABASE`, `BVC_DATABASE_HOST`,
`BVC_DATABASE_PORT`, `BVC_DATABASE_USERNAME`, `BVC_DATABASE_PASSWORD`,
`BVC_DATABASE_SSL_MODE`, `BVC_DATABASE_SSL_ROOT_CERT`, `BVC_DATABASE_SSL_CERT`,
`BVC_DATABASE_SSL_KEY`.

## Migrating an existing embedded-config

The flat keys no longer work. The mod refuses to start in embedded mode and logs
each one with the path that replaces it.

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

Before:

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

Note that `broadcast-range` never worked. The mod sent it at a path no server
field matched, so it was discarded and every embedded server ran at the default
of 48. Setting it under `voice.spatial_audio` applies it for the first time.

## For contributors

The classes in `common/src/main/kotlin/com/alaydriem/bedrockvoicechat/config/generated/`
are generated from the Rust `ApplicationConfig`. **Do not edit them.** Adding a
field to the server config and not regenerating fails the drift test.

Any mise task that builds the server regenerates them first, so a normal
`mise run build-server` keeps them current. To regenerate on their own:

```bash
mise run kotlin-config
```

CI runs the same test in its reporting direction (`mise run kotlin-config-check`),
which fails on a stale mirror instead of rewriting it.

The generator lives in `server/server/src/config/kotlin_export/`. Everything in
the server config is exported by default; the carve-outs are an explicit list in
`KotlinExporter::CARVE_OUTS`, and a carve-out path that no longer resolves to a
real field is a hard error rather than a silent leak.

### Tests

```bash
cd mods/java && ./gradlew build
cd mods/java/fabric && ./gradlew build
```
