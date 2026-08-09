---
title: WebSocket API
description: Drive mute, deafen, and recording from your own tooling.
sidebar:
  label: WebSocket API
  order: 4
---

BVC runs a local WebSocket server. External tooling can toggle mute, deafen and recording, hold push-to-talk, and start or stop a Proxy or Realm session. The [Stream Deck plugin](/wiki/creator/stream-deck/) uses it.

## Reference

**[bedrockvoicechat.com/websocket](https://www.bedrockvoicechat.com/websocket)**

## Turn it on

Open **Settings → WebSocket server**. Off by default.

Note the port and the authentication key. Every message carries the key. Messages without a valid one are rejected.

<div class="shot"><span>WebSocket settings page with the server enabled, showing port and key</span></div>

## Message shape

JSON, one object per message, with an `action` field:

```json
{ "action": "mute", "device": "input", "key": "<your key>" }
```

Eight actions: `ping`, `mute`, `record`, `state`, `ptt`, `targets`, `connect`, `disconnect`.

**Deafen is not its own action.** It is `mute` with `device: "output"`. `input` is your microphone, `output` is everything incoming.

## State

`mute`, `record`, `ptt`, `connect`, and `disconnect` push the new state to every connected client, not only the sender. A Stream Deck and your own tool stay in sync without polling.

Send `state` on connect for the current picture. It reports `muted`, `deafened`, `recording`, `voice_mode`, `ptt_active`, and `connection`.

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

## Connect to a world

`targets` lists the worlds this client can join: saved servers, servers the BVC server advertises, and your Realms. Each entry has an opaque `id`, a `name`, and a `kind` of `proxy` or `realm`.

`connect` takes an `id` from that list. **Quote the id back, never the list position.** Listing and connecting are two calls, and an entry added between them shifts the positions.

`disconnect` stops the live session. It names no target. Idempotent, and safe to send when nothing is connected.

Both reply with `connected`, plus `id` and `name` when there was a session. `state.connection` carries the same on every push.

## Security

The key is the only thing protecting the socket. Anyone who can reach the port and has the key can mute you or start a recording.

Keep it on your LAN. Do not port-forward it.

## Recording availability

`record` does nothing on a client that cannot record. Recording is desktop only.

It also does nothing when the connected server sets `voice.recording.enabled = false`. The client refuses to arm and returns the unchanged state. See [recording](/wiki/creator/recording/).
