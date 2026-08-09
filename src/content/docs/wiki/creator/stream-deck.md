---
title: Stream Deck
description: Control mute, deafen, and recording from an Elgato Stream Deck.
sidebar:
  label: Stream Deck
  order: 3
---

The Stream Deck plugin drives BVC over its local WebSocket API. Three actions: **mute**, **deafen**, **record**.

Download from the [Elgato Marketplace](https://marketplace.elgato.com/product/bedrock-voice-chat-c5a151d6-3669-487f-9548-bfe689e50203). Releases and issues are in [its own repository](https://github.com/Alaydriem/bedrock-voice-chat-streamdeck).

## Enable the WebSocket server

BVC does not listen by default. Open **Settings → WebSocket server**.

<div class="shot"><span>WebSocket settings page with the server enabled, showing port and key</span></div>

Note the port and the authentication key.

## Configure the plugin

Install the plugin, then enter the same port and authentication key. Set the host if BVC is not on the same machine as the Stream Deck.

<div class="shot"><span>Stream Deck plugin settings with port, host, and key filled in</span></div>

## The actions

| Action | Controls |
|---|---|
| Mute | Your microphone |
| Deafen | All incoming audio |
| Record | Session recording |

Each has on, off, and disconnected art. Disconnected means the plugin cannot reach BVC. Check that the app is running and the WebSocket server is on.

## Mute and push-to-talk

The mute action is a toggle, and a toggle applies in open-mic mode only. With BVC set to push to talk, the mute key will not open or close the microphone. BVC refuses the command.

Deafen works in both modes.

The API underneath has a held push-to-talk action and actions to list and connect to worlds. Whether the plugin exposes them depends on its own release. The [WebSocket API](/wiki/creator/websocket-api/) is the authority on what BVC accepts.

## Recording availability

Recording is desktop only. The record action does nothing against a mobile client, or against a server that has [turned recording off](/wiki/creator/recording/).

## Running across machines

With BVC on your gaming PC and the Stream Deck on a streaming PC, set the host to the gaming PC's LAN address and open the port.

The authentication key is the only thing protecting the socket. Do not expose it beyond your LAN.

## Building your own

The same API is available to anything that speaks WebSocket. See [WebSocket API](/wiki/creator/websocket-api/).
