---
title: OBS Overlay
description: Show who you can hear, and who is talking, as a Browser source in OBS.
sidebar:
  label: OBS Overlay
  order: 4
---

Bedrock Voice Chat provides an overlay for OBS. Add it as a Browser source to show viewers who you can hear and who is talking. The overlay has a transparent background. It shows a list of players and the BVC logo in the corners of the screen. By default, both are on the left side.

The overlay shows a square for each member of your group, and for each nearby player whose microphone is on.

## Add the source

1. In Bedrock Voice Chat, open **Settings → WebSocket server**.
2. Copy the **OBS overlay** URL.
3. In OBS, add a **Browser** source.
4. Paste the URL.
5. Set **Width** to `3840` and **Height** to `2160`.

## Options

Add these query parameters to the URL:

| Option | Default | Values | Effect |
| --- | --- | --- | --- |
| `layout` | `rail` | `rail`, `bar`, `chips` | A vertical column, a horizontal row, or small pills. |
| `anchor` | `tl` | `tl`, `tr`, `bl`, `br` | The corner for the squares. |
| `mark` | `bl` | `tl`, `tr`, `bl`, `br`, `off` | The corner for the logo. |
| `pulse` | `0` | `0`, `1` | `0` shows the logo at full height. `1` makes the logo rise and fall with the loudest speaker. |
| `self` | `0` | `0`, `1` | Whether you are in the list. |
| `faces` | `1` | `0`, `1` | Whether to show gamerpics. `0` always shows a letter. |
| `sources` | `1` | `0`, `1` | Whether group members look different from nearby players. |
| `quiet` | `1` | `0`, `1` | Whether silent players keep a square. `0` shows only the players who are talking. |
| `sort` | `name` | `name`, `activity` | `name` keeps the squares in a fixed order. `activity` moves the player who is talking to the front. |

For example, a horizontal row in the bottom-right corner that shows only the players who are talking:

```
http://127.0.0.1:9595/overlay?key=YOURKEY&layout=bar&anchor=br&quiet=0
```

## Size

The overlay is drawn for a 3840 by 2160 source. It scales to any scene size. One source works for 1080p and 4K canvases.

Use 3840 by 2160 in OBS, also when you stream at 1080p. An overlay that OBS scales down is sharper than one that OBS scales up.

## Blank overlay

- **Check the key.** If the key is wrong, the page shows a message instead of the overlay. Open the URL in a normal browser to read the message.
- **Check the port.** If a different program uses the port, Bedrock Voice Chat uses a different port. The URL in **Settings → WebSocket server** always contains the port in use.
- **Check that Bedrock Voice Chat is running.** The overlay dims when the connection to the app stops. It connects again when the app starts.
- **Use the URL from Settings.** The overlay must load from `127.0.0.1`. A copy of the page on a website cannot connect to the app.
