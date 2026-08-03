---
title: Stream Deck
description: Control mute, deafen, and recording from an Elgato Stream Deck.
sidebar:
  label: Stream Deck
  order: 3
---

The Stream Deck plugin drives BVC over its local WebSocket API. Three actions: **mute**, **deafen**, **record**.

Download from the [Elgato Marketplace](https://marketplace.elgato.com/product/bedrock-voice-chat-c5a151d6-3669-487f-9548-bfe689e50203). Releases and issues live in [its own repository](https://github.com/Alaydriem/bedrock-voice-chat-streamdeck).

## Enable the WebSocket server

BVC does not listen by default. Settings → WebSocket.

<div class="shot"><span>WebSocket settings page with the server enabled, showing port and key</span></div>

Note the port and the authentication key.

## Configure the plugin

Install the plugin, then mirror the same settings in it: port, authentication key, and host if BVC is not on the same machine as the Stream Deck.

<div class="shot"><span>Stream Deck plugin settings with port, host, and key filled in</span></div>

## The actions

| Action | Controls |
|---|---|
| Mute | Your microphone |
| Deafen | All incoming audio |
| Record | Session recording |

Each has on, off, and disconnected art, so the key reflects actual state rather than what you last pressed. Disconnected means the plugin cannot reach BVC — check that the app is running and the WebSocket server is on.

Recording is desktop only, so the record action does nothing against a mobile client.

## Running across machines

If BVC is on your gaming PC and the Stream Deck is on a streaming PC, set the host to the gaming PC's LAN address and make sure the port is reachable.

The authentication key is the only thing protecting the socket. Anyone who can reach the port and has the key can mute you, so do not expose it beyond your LAN.

## Building your own

The same API is available to anything that speaks WebSocket. See [WebSocket API](/wiki/creator/websocket-api/).
