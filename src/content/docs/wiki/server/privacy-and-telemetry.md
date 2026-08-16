---
title: Privacy and telemetry
description: What BVC collects, how to turn it off, and setting a minimum age for your community.
sidebar:
  label: Privacy and telemetry
  order: 9
---

Bedrock Voice Chat collects a small amount of anonymous operational telemetry. You can turn it off entirely.

## Telemetry

Bedrock Voice Chat collects anonymous operational metrics: connection counts, session lifecycle events, and error rates.

It does not collect gamertags, message content, audio, or position data.

**It is enabled by default.** To turn it off:

```hcl
server {
    features {
        telemetry = false
    }
}
```

Or set `BVC_TELEMETRY=false`.

Turning it off costs you nothing functionally. It costs the project visibility into crashes and regressions across real deployments. Your server, your call.

## Your own metrics

`/metrics` always serves Prometheus-format metrics regardless of the `telemetry` flag, and the health endpoints are always available. Those are local to your deployment and go nowhere.

See [monitoring](/wiki/server/monitoring/).

## Minimum age

Set a minimum age for your community and the client apps will present Age Signals (Android) or Declared Age Ranges (iOS) prompts to players in regions where those are required.

```hcl
server {
    age {
        minimum = 13
    }
}
```

| Value | Effect |
|---|---|
| `0` | Enforcement off. |
| `13`–`18` | Enforced at that age. |
| Anything else | Clamped into 13–18. |
| Unset | Defaults to 13. |

The prompt only appears when both are true: you have set a minimum, and the player's device and account are in a region where the platform requires it. Setting a minimum does nothing visible to players elsewhere.

BVC only asks the platform whether the signed-in user clears your minimum. It does not receive, store, or transmit anyone's age.

Parent-facing explanation: [parents and age settings](/wiki/player/parents-and-age/). Worth linking from your own rules page if you run a server with younger players.

## Player-to-player controls

Players can mute each other locally. Muting is one-directional. It stops you hearing them, not them hearing you.

There is no server-side moderation tooling for voice. You can banish a player entirely with `bvc user banish`, and that is the extent of it. Anything more nuanced is a matter for your own community rules and moderators.

Groups are not private from their members. Everyone in a group hears everything, and any member can be recording locally.

## Full privacy policy

[PRIVACY_STATEMENT.md](https://github.com/Alaydriem/bedrock-voice-chat/blob/master/PRIVACY_STATEMENT.md)
