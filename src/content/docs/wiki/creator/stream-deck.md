---
title: Stream Deck
description: Control mute, deafen, recording, jukebox music, world connection, and live stats from an Elgato Stream Deck.
sidebar:
  label: Stream Deck
  order: 3
---

The Bedrock Voice Chat Stream Deck plugin lets you drive the app from an Elgato Stream Deck, with six actions: **Microphone**, **Deafen**, **Record**, **Jukebox**, **Connect**, and **Stats**.

Download it from the [Elgato Marketplace](https://marketplace.elgato.com/product/bedrock-voice-chat-c5a151d6-3669-487f-9548-bfe689e50203). Releases and issues are in [its own repository](https://github.com/Alaydriem/bedrock-voice-chat-streamdeck).

## Enable the WebSocket server

Bedrock Voice Chat does not listen by default. Open **Settings → WebSocket server**.

<div class="shot"><span>WebSocket settings page with the server enabled, showing port and key</span></div>

Note the port and the authentication key.

## Configure the plugin

Install the plugin, then enter the same port and authentication key. Set the host if BVC is not on the same machine as the Stream Deck.

<div class="shot"><span>Stream Deck plugin settings with port, host, and key filled in</span></div>

## The actions

| Action | Controls | Needs setup |
|---|---|---|
| Microphone | Your microphone | — |
| Deafen | All incoming audio | — |
| Record | Session recording | — |
| Jukebox | Jukebox music | — |
| Connect | The world BVC is connected to | Pick the world |
| Stats | Nothing. Draws one live number | Pick the stat |

Each has on, off, and disconnected art. Disconnected means the plugin cannot reach BVC. Check that the app is running and the WebSocket server is on.

Set up the last two from the key's own settings in the Stream Deck app.

## Microphone and push-to-talk

The Microphone key follows the voice mode. In open mic it toggles mute. In push to talk it opens the microphone on press and closes it on release.

Deafen and Jukebox work in both modes.

## Jukebox

Toggles all jukebox music for you alone. It changes nothing for anyone else in range, and does not touch voices.

The key mutes and unmutes. Set the level in the app, or in game with `/bvc:jukeboxvolume` and `/bvc jukebox volume`. See [using the jukebox](/wiki/player/using-the-jukebox/).

## Connect

Pick a Realm or proxy in the key's settings. Pressing the key connects to it. Pressing it while connected disconnects.

The list comes from BVC. Have the app running and reachable before you open the settings.

## Stats

One number per key. Add a second Stats key for a second number.

Choose from the list in the key's settings. It is built from what BVC is reporting right now. Press refresh if it is empty.

A number goes to `—` when it is more than three seconds old, and when the plugin loses its connection.

Stats arrive on the WebSocket server's `/metrics` route. Enabling the server covers both.

## Beyond the plugin

BVC also accepts jukebox volume as a level, and group create, join and leave. The plugin has no keys for those. The [WebSocket API](/wiki/creator/websocket-api/) is the authority on what BVC accepts.

## Recording availability

Recording is desktop only. The record action does nothing against a mobile client, or against a server that has [turned recording off](/wiki/creator/recording/).

## Running across machines

With BVC on your gaming PC and the Stream Deck on a streaming PC, set the host to the gaming PC's LAN address and open the port.

The authentication key is the only thing protecting the socket. Do not expose it beyond your LAN.

## Building your own

The same API is available to anything that speaks WebSocket. See [WebSocket API](/wiki/creator/websocket-api/).
