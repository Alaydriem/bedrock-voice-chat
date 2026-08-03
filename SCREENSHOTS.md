# Screenshot checklist

25 placeholders across 14 pages, grouped so each group is one capture session.

Regenerate this list from the source at any time:

```bash
grep -rn 'class="shot"' src/content/docs/wiki/
```

## How to fill one

Drop the image in `public/assets/wiki/` and replace the whole `<div class="shot">…</div>`
with an image tag:

```html
<img src="/assets/wiki/signing-in-login.png" alt="BVC login window" />
```

Delete the placeholder div when you do. An unfilled slot renders as a dashed box on the
live page — deliberately visible, so it does not ship unnoticed.

PNG. Under ~200 KB where you can; Astro does not optimise anything in `public/`.

---

## Group 1 — BVC client, signed in (7)

One session in the desktop app with a couple of players nearby and a group active.

- [ ] `player/signing-in` — Login window, server URL field filled in
- [ ] `player/signing-in` — Dashboard immediately after a successful sign-in
- [ ] `player/using-voice-chat` — Dashboard with the player list showing proximity and group members
- [ ] `player/groups` — Group panel in the left sub-sidebar, showing members
- [ ] `creator/recording` — REC button active, red recording indicator visible
- [ ] `player/settings` — Audio settings page, input and output device dropdowns
- [ ] `player/audio-library` — Audio Library settings page with several uploaded clips

Shots 2 and 3 may be the same frame if the dashboard already shows players.

## Group 2 — Client settings, remaining (5)

Same app, settings only. No game needed.

- [ ] `player/settings` — Recordings settings page listing recent sessions
- [ ] `creator/recording` — Recordings page with a session expanded to show participants
- [ ] `creator/stream-deck` — WebSocket settings page, server enabled, port and key visible
- [ ] `creator/websocket-api` — Same shot as above, reused
- [ ] `platforms/version-support` — Proxy Connect server entry, advertised-version dropdown open

**Redact the authentication key.** The WebSocket shot appears on two pages; one capture
covers both.

## Group 3 — Proxy Connect and Realms Connect (4)

Needs a configured Aternos server and a Realm.

- [ ] `platforms/aternos` — Proxy Connect add-server dialog with an Aternos address
- [ ] `platforms/aternos` — Proxy Connect showing connection info for a connected server
- [ ] `platforms/realms` — Realms Connect page with a Realm selected
- [ ] `platforms/realms` — Connection information shown after selecting a Realm

**Redact server hostname and LAN IP** in the two connection-info shots.

## Group 4 — Minecraft Bedrock client, world creation (3)

One pass through building the no-net world.

- [ ] `server/nonet-addon` — Behavior Packs list with the Realms + No-Net pack enabled
- [ ] `server/nonet-addon` — Experiments toggle showing Beta APIs enabled
- [ ] `server/nonet-addon` — World settings with Export World visible at the bottom

Old wiki equivalents are still live and reusable if the UI has not changed:
`34b0b333`, `f8489b65`, `9cf88647` under `github.com/user-attachments/assets/`.

## Group 5 — Aternos web UI (2)

- [ ] `platforms/aternos` — World upload page
- [ ] `platforms/aternos` — Server version selector

Old wiki equivalents: `134b3630`, `d8c1be05`.

## Group 6 — In-game, Bedrock (3)

Needs a BDS server running the Addon.

- [ ] `player/in-game-commands/bedrock` — `/bvc:panel` control panel open in-game
- [ ] `player/using-the-jukebox` — Inventory showing a BVC disc with its `BVC: <audio_id>` name
- [ ] `player/using-the-jukebox` — Crafting recipe for the BVC audio player block

For the disc, run `/bvc:disc <id>` against a clip that exists so the name renders.

## Group 7 — Stream Deck (1)

- [ ] `creator/stream-deck` — Plugin settings with port, host, and key filled in

**Redact the key.**

---

## Notes

**Four old-wiki images are already dead.** The `private-user-images` embeds on the sign-in
and player-audio pages expired 2025-10-21. Group 1 replaces them, and the client UI has
moved on since they were taken.

**Still-good old images** use `github.com/user-attachments/assets/` URLs and are stable —
Groups 4 and 5 could reuse them rather than being recaptured.

**Redact before publishing**: WebSocket authentication keys, server hostnames, LAN IPs,
and your gamertag if you would rather it not appear.
