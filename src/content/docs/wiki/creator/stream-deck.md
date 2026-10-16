---
title: Stream Deck
description: Control mute, deafen, recording, jukebox music, world connection, and live stats from an Elgato Stream Deck.
sidebar:
  label: Stream Deck
  order: 3
---

The Bedrock Voice Chat Stream Deck plugin lets you control the app from an Elgato Stream Deck. It has six actions: **Microphone**, **Deafen**, **Record**, **Jukebox**, **Connect**, and **Stats**.

Download it from the [Elgato Marketplace](https://marketplace.elgato.com/product/bedrock-voice-chat-c5a151d6-3669-487f-9548-bfe689e50203). Releases and issues are in the [plugin repository](https://github.com/Alaydriem/bedrock-voice-chat-streamdeck).

## Enable the WebSocket server

The WebSocket server is off by default. Open **Settings → WebSocket server** and turn it on.

<div class="window"><img src="/assets/wiki/settings-websocket.jpg" alt="WebSocket settings page with the server enabled, showing port and key" /></div>

Write down the port and the authentication key.

## Configure the plugin

Install the plugin, then enter the same port and authentication key. If BVC is on a different computer from the Stream Deck, also set the host.

## Actions

| Action | Controls | Setup |
|---|---|---|
| Microphone | Your microphone. | None |
| Deafen | All incoming audio. | None |
| Record | Session recording. | None |
| Jukebox | Jukebox music. | None |
| Connect | The world that BVC is connected to. | Select the world. |
| Stats | Nothing. Shows one live number. | Select the stat. |

Each key has an image for on, off, and disconnected. Disconnected means that the plugin cannot connect to BVC. Make sure that the app is running and the WebSocket server is on.

Set up Connect and Stats in each key's settings in the Stream Deck app.

## Microphone and push-to-talk

The Microphone key follows the voice mode. In open mic mode, it mutes and unmutes. In push-to-talk mode, it opens the microphone when you press it and closes the microphone when you release it.

Deafen and Jukebox work in both modes.

## Jukebox

The Jukebox key mutes or unmutes all jukebox music for you. Other players and voice audio are not affected.

To set the jukebox volume, use the app, or use the in-game command: `/bvc:jukeboxvolume` on Bedrock or `/bvc jukebox volume` on Java. See [using the jukebox](/wiki/player/using-the-jukebox/).

## Connect

Select a Realm or a server in the key's settings. Press the key to connect to it. Press the key again to disconnect.

The list comes from BVC. Start the app before you open the key's settings.

## Stats

Each Stats key shows one number. To show a second number, add a second Stats key.

Select the stat from the list in the key's settings. The list contains the values that BVC sends now. If the list is empty, press refresh.

A number changes to `—` when it is more than three seconds old, or when the plugin loses its connection.

Stats come from the `/metrics` route of the WebSocket server. When you turn on the WebSocket server, commands and stats both work.

## Other actions

BVC also accepts a jukebox volume level, and group create, join and leave. The plugin has no keys for these actions. The [WebSocket API](/wiki/creator/websocket-api/) page lists all actions.

## Recording availability

Recording is available only on desktop. The Record key does nothing when BVC runs on a mobile device, or when the server has [turned recording off](/wiki/creator/recording/).

## Two computers

If BVC runs on your gaming PC and the Stream Deck is on a streaming PC, set the host to the LAN address of the gaming PC. Allow the port through the firewall on the gaming PC.

Only the authentication key protects the WebSocket server. Do not make the port reachable from outside your LAN.

## Your own tools

Any program that can use WebSocket can use the same API. See [WebSocket API](/wiki/creator/websocket-api/).
