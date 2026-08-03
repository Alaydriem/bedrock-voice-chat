---
title: WebSocket API
description: Drive mute, deafen, and recording from your own tooling.
sidebar:
  label: WebSocket API
  order: 4
---

BVC runs a local WebSocket server so external tooling can toggle mute, deafen, and recording. It is what the [Stream Deck plugin](/wiki/creator/stream-deck/) uses.

## Reference

**[bedrockvoicechat.com/websocket](https://www.bedrockvoicechat.com/websocket)**

Generated from the source on every build, so it always matches the shipped client. Message shapes, fields, and responses live there rather than being restated here — a copy on this page would drift, and you would have no way to tell which one was lying.

What follows is orientation, not a substitute.

## Turning it on

Settings → WebSocket. Off by default.

Note the port and the authentication key. Every message carries the key, and messages without a valid one are rejected.

<div class="shot"><span>WebSocket settings page with the server enabled, showing port and key</span></div>

## Shape of it

JSON, one object per message, with an `action` field selecting the command:

```json
{ "action": "mute", "device": "input", "key": "<your key>" }
```

Four actions: `ping`, `mute`, `record`, `state`.

The one thing worth knowing before you read the reference: **deafen is not its own action.** It is `mute` with `device: "output"` — `input` is your microphone, `output` is everything incoming.

## State is broadcast

`mute` and `record` change state, and the new state is pushed to every connected client rather than only the sender. A Stream Deck and your own tool stay in sync without either polling.

Use `state` to get the current picture when you connect.

## Security

The key is the only thing protecting the socket. Anyone who can reach the port and has it can mute you or start a recording.

Keep it on your LAN. Do not port-forward it.

## Recording is desktop only

The `record` action exists on every platform and does nothing on a client that cannot record.
