---
title: OBS Overlay
description: Show who you can hear, and who is talking, as a Browser source in OBS.
sidebar:
  label: OBS Overlay
  order: 4
---

Bedrock Voice Chat provides an OBS overlay creators can use as a OBS browser source to show viewers who you are talking to. The OBS overlay is a transparent overlay that lists players, and the BVC logo along the left side, unobtrusivly and out of the way from your main stream.

The overlay shows squares for every member in your group, and players within voice proximity that have active microphones.

## Add the source

1. In Bedrock Voice Chat, open **Settings → WebSocket**.
2. Copy the **OBS overlay** URL.
3. In OBS, add a **Browser** source.
4. Paste the URL.
5. Set **Width** to `3840` and **Height** to `2160`.

## Options

The following query parameters are available to customize the overlay

| Option | Default | Values | Effect |
| --- | --- | --- | --- |
| `layout` | `rail` | `rail`, `bar`, `chips` | A vertical column, a horizontal row, or small pills. |
| `anchor` | `tl` | `tl`, `tr`, `bl`, `br` | Which corner holds the squares. |
| `mark` | `bl` | `tl`, `tr`, `bl`, `br`, `off` | Which corner holds the logo. |
| `pulse` | `0` | `0`, `1` | `0` draws the logo at full height. `1` makes it rise and fall with the loudest speaker. |
| `self` | `0` | `0`, `1` | Whether you appear. |
| `faces` | `1` | `0`, `1` | Whether to show gamerpics. `0` always shows a letter. |
| `sources` | `1` | `0`, `1` | Whether to mark group members apart from neighbours. |
| `quiet` | `1` | `0`, `1` | Whether silent people keep a square. `0` shows only who is talking. |
| `sort` | `name` | `name`, `activity` | `name` keeps squares still. `activity` moves the talker to the front. |

For example, a horizontal row in the bottom-right that shows only who is talking:

```
http://127.0.0.1:9595/overlay?key=YOURKEY&layout=bar&anchor=br&quiet=0
```

## Size

The overlay is drawn for a 3840 by 2160 source. It scales to any scene, so one source works for
a 1080p canvas and a 4K canvas. Use 3840 by 2160 in OBS even when you stream at 1080p: OBS then
scales the overlay down, which is sharper than scaling it up.

## If the overlay is blank

- **Check the key.** A wrong key gives a message instead of the overlay. Open the URL in a normal
  browser to read it.
- **Check the port.** Bedrock Voice Chat moves its listener when something else holds the port.
  The URL in **Settings → WebSocket** always names the port in use.
- **Check that Bedrock Voice Chat is running.** The overlay dims when it loses the connection. It
  recovers by itself when the app comes back.
- **Use the URL from Settings.** The overlay must load from `127.0.0.1`. A copy of the page on a
  website cannot reach the app: browsers block a public page from connecting to your own computer.
