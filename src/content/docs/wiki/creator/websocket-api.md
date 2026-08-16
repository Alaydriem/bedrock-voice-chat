---
title: WebSocket API
description: Drive mute, deafen, recording, jukebox music, and groups from your own tooling.
sidebar:
  label: WebSocket API
  order: 4
---

Bedrock Voice Chat runs a local WebSocket server that lets your own tooling toggle mute, deafen and recording, set jukebox music levels, manage groups, hold push-to-talk, and start or stop a Bedrock Voice Chat Connect session. The [Stream Deck plugin](/wiki/creator/stream-deck/) uses it.

## Reference

**[bedrockvoicechat.com/websocket](https://www.bedrockvoicechat.com/websocket)**

## Turn it on

Open **Settings → WebSocket server**. Off by default.

Note the port and the authentication key. Every message carries the key. Messages without a valid one are rejected.

<div class="shot"><span>WebSocket settings page with the server enabled, showing port and key</span></div>

## Routes

Two endpoints on the same port.

| Path | Direction | Authentication |
|---|---|---|
| Any path except `/metrics` | Request and response | Per message, as `key` |
| `/metrics` | Push only | At the upgrade, as `?key=` |

Only the exact path `/metrics` is routed to the diagnostics stream. Everything else reaches the command protocol, including `/` and `/ws`. An integration that connects on an arbitrary path keeps working.

## Message shape

JSON, one object per message, with an `action` field:

```json
{ "action": "mute", "device": "input", "key": "<your key>" }
```

Thirteen actions: `ping`, `mute`, `record`, `jukebox`, `jukeboxvolume`, `state`, `ptt`, `targets`, `connect`, `disconnect`, `creategroup`, `joingroup`, `leavegroup`.

**Deafen is not its own action.** It is `mute` with `device: "output"`. `input` is your microphone, `output` is everything incoming.

## State

`mute`, `record`, `jukebox`, `jukeboxvolume`, `ptt`, `connect`, `disconnect`, and the three group actions push the new state to every connected client, not only the sender. A Stream Deck and your own tool stay in sync without polling.

Send `state` on connect for the current picture. It reports `muted`, `deafened`, `recording`, `voice_mode`, `ptt_active`, `jukebox_muted`, `jukebox_gain`, and `connection`.

## Voice mode

`state` carries `voice_mode`, either `openMic` or `pushToTalk`. **Read it before you draw a microphone button.**

| Mode | Input mute | `ptt` |
|---|---|---|
| `openMic` | Works | Refused |
| `pushToTalk` | Refused | Controls the microphone |

Both refusals return an error naming the reason. An input mute in `pushToTalk` is told to send `ptt` instead.

Output mute always works.

### Push-to-talk

`ptt` takes `down`: `true` on the press, `false` on the release.

Send the release. A dropped connection does not close the microphone, and a button that latches leaves it open.

## Jukebox music

One setting covers every jukebox the client is playing. Voices are unaffected.

| Action | Takes | Does |
|---|---|---|
| `jukebox` | nothing | Toggles mute. |
| `jukeboxvolume` | `level`, 0–150 | Sets the level. |

`jukebox` is a toggle. A button cannot read the current state before it is pressed.

Both reply with `muted` and `gain`. **`level` goes in as a percent. `gain` comes back as a fraction.** Send 150, read back 1.5.

Mute and level are independent. A level set while muted returns on unmute.

## Groups

| Action | Takes | Does |
|---|---|---|
| `creategroup` | `name` | Creates a group and joins it. |
| `joingroup` | `name` | Joins an existing group. |
| `leavegroup` | nothing | Leaves the current group. |

All three reply with `in_group`, plus `id` and `name` when in one.

Creating and joining are moves. The client leaves its current group first.

`joingroup` is refused when the name matches no group, and when it matches more than one. Matching ignores case.

## Connect to a world

`targets` lists the worlds this client can join: saved servers, servers the BVC server advertises, and your Realms. Each entry has an opaque `id`, a `name`, and a `kind` of `proxy` or `realm`.

`connect` takes an `id` from that list. **Quote the id back, never the list position.** Listing and connecting are two calls, and an entry added between them shifts the positions.

`disconnect` stops the live session. It names no target. Idempotent, and safe to send when nothing is connected.

Both reply with `connected`, plus `id` and `name` when there was a session. `state.connection` carries the same on every push.

## Metrics stream

A push-only diagnostics feed. There is no request. Connect and read.

```
ws://127.0.0.1:<port>/metrics?key=<your key>
```

The key goes on the query string. There is no inbound message to carry it. A missing or wrong key is refused at the upgrade.

A client with no key configured accepts the connection without one.

Each push carries microphone, playback, link quality, session and transport diagnostics, with round-trip time and loss thresholds. The payload shape is on the [reference](https://www.bedrockvoicechat.com/websocket).

The command protocol is unaffected. It still authenticates per message.

## Security

:::danger
The key is the only thing protecting the socket. Anyone who can reach the port and has the key can mute you, start a recording, or read your diagnostics stream.

Keep it on your LAN. Do **not** port-forward it.
:::

## Recording availability

`record` does nothing on a client that cannot record. Recording is desktop only.

It also does nothing when the connected server sets `voice.recording.enabled = false`. The client refuses to arm and returns the unchanged state. See [recording](/wiki/creator/recording/).
