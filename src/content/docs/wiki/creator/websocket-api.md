---
title: WebSocket API
description: Drive mute, deafen, recording, jukebox music, and groups from your own tooling.
sidebar:
  label: WebSocket API
  order: 5
---

Bedrock Voice Chat runs a local WebSocket server. Your own tools can use it to toggle mute, deafen and recording, set the jukebox music level, manage groups, hold push-to-talk, and start or stop a Bedrock Voice Chat Connect session. The [Stream Deck plugin](/wiki/creator/stream-deck/) uses this server.

## Reference

The full reference is at [bedrockvoicechat.com/websocket](https://www.bedrockvoicechat.com/websocket).

## Enable the server

The WebSocket server is off by default. Open **Settings → WebSocket server** and turn it on.

Write down the port and the authentication key. Every message must contain the key. The server rejects a message without a valid key.

<div class="window"><img src="/assets/wiki/settings-websocket.jpg" alt="WebSocket settings page with the server enabled, showing port and key" /></div>

## Routes

The server has two endpoints on the same port.

| Path | Direction | Authentication |
|---|---|---|
| Any path except `/metrics` | Request and response | Per message, as `key` |
| `/metrics` | Push only | At the upgrade, as `?key=` |

Only the exact path `/metrics` goes to the diagnostics stream. All other paths, including `/` and `/ws`, go to the command protocol. An integration that connects on any other path continues to work.

## Message format

Each message is one JSON object with an `action` field:

```json
{ "action": "mute", "device": "input", "key": "<your key>" }
```

There are thirteen actions: `ping`, `mute`, `record`, `jukebox`, `jukeboxvolume`, `state`, `ptt`, `targets`, `connect`, `disconnect`, `creategroup`, `joingroup`, `leavegroup`.

**Deafen is not a separate action.** To deafen, send `mute` with `device: "output"`. `input` is your microphone. `output` is all incoming audio.

## State

`mute`, `record`, `jukebox`, `jukeboxvolume`, `ptt`, `connect`, `disconnect`, and the three group actions send the new state to every connected client. A Stream Deck and your own tool stay in sync without polling.

Send `state` after you connect to get the current state. It reports `muted`, `deafened`, `recording`, `voice_mode`, `ptt_active`, `jukebox_muted`, `jukebox_gain`, and `connection`.

## Voice mode

`state` contains `voice_mode`, either `openMic` or `pushToTalk`. **Read it before you draw a microphone button.**

| Mode | Input mute | `ptt` |
|---|---|---|
| `openMic` | Works | Refused |
| `pushToTalk` | Refused | Controls the microphone |

Both refusals return an error that gives the reason. If you send an input mute in `pushToTalk`, the error tells you to send `ptt`.

Output mute always works.

### Push-to-talk

`ptt` takes `down`: `true` when the button is pressed, `false` when it is released.

Always send the release. A lost connection does not close the microphone. A button that stays down keeps the microphone open.

## Jukebox music

One setting controls all jukeboxes that the client plays. Voices are not affected.

| Action | Takes | Does |
|---|---|---|
| `jukebox` | nothing | Toggles mute. |
| `jukeboxvolume` | `level`, 0–150 | Sets the level. |

`jukebox` toggles mute. To get the current state before you draw a button, send `state` and read `jukebox_muted`.

Both actions reply with `muted` and `gain`. **Send `level` as a percentage. `gain` comes back as a fraction.** If you send 150, you get 1.5.

Mute and level are separate. A level that you set while muted applies when you unmute.

## Groups

| Action | Takes | Does |
|---|---|---|
| `creategroup` | `name` | Creates a group and joins it. |
| `joingroup` | `name` | Joins an existing group. |
| `leavegroup` | nothing | Leaves the current group. |

All three reply with `in_group`. If the client is in a group, the reply also has `id` and `name`.

`creategroup` and `joingroup` first remove the client from its current group.

`joingroup` is refused if the name matches no group, or more than one group. The match ignores case.

For a [Private Group](/wiki/player/groups/#private-groups), `joingroup` opens the join dialog in the app and sends the request. The client joins only after a member admits it.

## Connect to a world

`targets` lists the worlds that this client can join: saved servers, servers that the BVC server advertises, and your Realms. Each entry has an opaque `id`, a `name`, and a `kind` of `proxy` or `realm`.

`connect` takes an `id` from that list. **Send the `id`, not the list position.** The list can change between `targets` and `connect`.

`disconnect` stops the live session. It takes no target. Idempotent; safe to send when nothing is connected.

Both reply with `connected`. If there was a session, the reply also has `id` and `name`. Every `state` message has the same values in `state.connection`.

## Metrics stream

The metrics stream sends diagnostics only. You do not send requests. Connect and read.

```
ws://127.0.0.1:<port>/metrics?key=<your key>
```

Put the key in the query string. You send no messages on this stream. If the key is missing or wrong, the connection is refused.

If no key is set in BVC, `/metrics` accepts connections without a key.

Each message contains microphone, playback, connection quality, session and transport diagnostics, with round-trip time and packet-loss values. The message format is in the [reference](https://www.bedrockvoicechat.com/websocket).

The command protocol does not change. Each command message still needs the key.

## Security

:::danger
Only the key protects the WebSocket server. Anyone who can reach the port and has the key can mute you, start a recording, or read your diagnostics stream.

Keep the port on your LAN. Do **not** forward it on your router.
:::

## Recording availability

`record` does nothing on a client that cannot record. Recording is available only on desktop.

`record` also does nothing when the connected server sets `voice.recording.enabled = false`. The client does not start a recording, and returns the current state. See [recording](/wiki/creator/recording/).
