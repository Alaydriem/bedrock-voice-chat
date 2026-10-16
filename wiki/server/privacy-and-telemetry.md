---
title: "Privacy and telemetry"
description: "What BVC collects, how to turn it off, and setting a minimum age for your community."
source: https://www.bedrockvoicechat.com/wiki/server/privacy-and-telemetry/
---
Bedrock Voice Chat collects a small amount of anonymous telemetry. You can turn it off.

## Telemetry

Bedrock Voice Chat collects anonymous operational metrics: connection counts, session start and end events, and error rates.

It does not collect gamertags, message content, audio, or position data.

Telemetry is on by default. To turn it off:

```hcl
server {
    features {
        telemetry = false
    }
}
```

You can also set `BVC_TELEMETRY=false`.

Turning telemetry off does not change how BVC works. The BVC developers then cannot see crashes and regressions on your server.

## Your own metrics

`/metrics` always serves Prometheus metrics, whatever the value of the `telemetry` flag. The health endpoints are always available. BVC does not send these metrics to anyone. Only your own deployment can read them.

See [monitoring](/wiki/server/monitoring/).

## Minimum age

Set a minimum age with `server.age.minimum`. In regions that require age checks, the client app then asks the player to confirm their age. Android uses Age Signals. iOS uses Declared Age Ranges.

```hcl
server {
    age {
        minimum = 13
    }
}
```

| Value | Effect |
|---|---|
| `0` | No age check. |
| `13` to `18` | Players must be at least this age. |
| Any other value | Changed to the nearest value from 13 to 18. |
| Not set | 13. |

The prompt appears only if the minimum is not `0` and the platform requires age checks in the player's region. Players in other regions see no change.

BVC asks the platform only if the signed-in player is at or above your minimum age. BVC does not receive, store, or send any player's age.

[Parents and age settings](/wiki/player/parents-and-age/) explains these settings for parents. If your server has younger players, link that page from your server rules.

## Player-to-player controls

Players can mute each other on their own device. When you mute a player, you do not hear them. They can still hear you.

BVC has no server-side moderation tools for voice. To remove a player from voice chat, use `bvc user banish`. For other moderation, use your community rules and moderators.

Group members can hear everything in the group. Any member can record the group on their own device.

## Full privacy policy

[PRIVACY_STATEMENT.md](https://github.com/Alaydriem/bedrock-voice-chat/blob/master/PRIVACY_STATEMENT.md)