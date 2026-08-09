# Bedrock Voice Chat — Java mods

Fabric and Paper builds of the BVC mod. Both share `:common`, and both read the
same configuration shape.

- Fabric: `config/bedrock-voice-chat.json`
- Paper: `plugins/BedrockVoiceChat/config.yml`

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
      certificate: "/etc/bvc/fullchain.pem"
      key: "/etc/bvc/privkey.pem"
      names:
        - "bvc.example.com"
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

**Bedrock relay.** `server.bedrock.enabled` defaults to **true** with
`transfer_port` **19132**, which Geyser normally owns on a Java server. The bind
failure is logged and the server keeps running, but the relay will not work. Set
`transfer_port: 19139` to coexist with Geyser, or `enabled: false` to turn the
relay off.

**DNS.** `server.bedrock.dns.enabled` defaults to false, so port 53 is not
touched unless you ask for it.

### TLS is required

Supply `server.tls.certificate` and `server.tls.key`, **or** a
`server.tls.acme` block to obtain a certificate over DNS-01. The mod refuses to
start with neither. Setting both is refused by the server itself.

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
`BVC_TELEMETRY`, `BVC_TLS_CERTIFICATE`, `BVC_TLS_KEY`, `BVC_TLS_CERTS_PATH`,
`BVC_TLS_NAMES`, `BVC_TLS_IPS`, `BVC_ACME_EMAIL`, `BVC_ACME_PROVIDER`,
`BVC_ACME_API_TOKEN`, `BVC_ACME_DIRECTORY`, `BVC_ACME_DOMAINS`,
`BVC_ACME_DNS_URL`, `BVC_ACME_DNS_USERNAME`, `BVC_ACME_DNS_PASSWORD`,
`BVC_ACME_DNS_SUBDOMAIN`, `BVC_BEDROCK_ENABLED`, `BVC_BEDROCK_TRANSFER_PORT`,
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

```bash
cd server/server
UPDATE_KOTLIN_CONFIG=1 cargo nextest run kotlin_export
```

The generator lives in `server/server/src/config/kotlin_export/`. Everything in
the server config is exported by default; the carve-outs are an explicit list in
`KotlinExporter::CARVE_OUTS`, and a carve-out path that no longer resolves to a
real field is a hard error rather than a silent leak.

### Tests

```bash
cd mods/java && ./gradlew build
cd mods/java/fabric && ./gradlew build
```
