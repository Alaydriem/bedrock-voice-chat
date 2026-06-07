# Spec: No-Net Jukebox Insert via `/bvc:play` Command Bus

> Fixes the no-net disc-swap bug and the no-net hopper-insert gap. The BDS mod becomes the authority for the insert decision and emits a `/bvc:play` command; the client proxy intercepts it and dispatches playback. Eject (player + track-end) reuses existing behavior. No server changes.

## Model (as specified)

1. **Mode branch.** Reuse existing net detection (`httpClient.ensureLoaded()` availability, already computed in `main.ts`). On the insert path, an `if` branch calls a **separate no-net TS module** instead of the net (HTTP) module.
2. **Insert → emit command (no-net).** On disc insertion (already detected in `components.ts` `onPlayerInteract` / `onTick`), the no-net module has the **mod emit** the command:
   ```
   /bvc:play "<player>" <audio_id> <x> <y> <z> <dimension>
   ```
   `"<player>"` is the player **name** (quoted; handles spaces) — matched against the proxy's `session.player.name`. Name only, no uuid (name is trivial on both ends; XUID isn't cleanly exposed in BDS scripting). It may be visible to players; **suppress if we can, visible is acceptable**.
3. **Proxy intercepts & dispatches.** Rust proxy reads `player_id`, `audio_id`, position from the command. **If the player matches this proxy's player**, it sends the `JukeboxInsert` QUIC packet to the BVC server to begin playback.
4. **Eject (player).** Right-clicking the jukebox again yields the **existing eject behavior** — `/bvc:eject` is emitted, audio stops, and **the player who initiates the eject handles it**.
5. **Hopper insert.** No actor → the **closest player** fulfills the `/bvc:play` (mod picks closest, names them in the command).
6. **Track end.** **Existing auto-eject behavior runs** unchanged (`EjectScheduler` → `JukeboxEjectAnnouncement` → `JukeboxEjectInjector` → `!bvce`/`/bvc:eject` → mod `forceEject`).

## Why this fixes the bugs
- **Swap:** the proxy no longer infers insert from sniffed `ItemUse` packets — it only acts on the mod's explicit `/bvc:play`. On an occupied right-click the mod ejects and never emits `/bvc:play`, so disc B never starts; held disc stays in hand. The swallowed-eject path (`note_insert_pending` vs the mod's eject `UpdateBlock`) is gone.
- **Hopper:** the mod already detects hopper inserts (`pullDiscFromAdjacentHopper`); it now emits `/bvc:play` for the closest player, so audio starts without needing a player `ItemUse` packet.

## Changes

### Mod (TS) — no-net module + if-branch
- New no-net audio module (class, own file per CLAUDE.md): formats and emits `/bvc:play`, picks the fulfiller (actor for clicks, closest player for hoppers).
- `AudioPlayerManager` (or the interact/tick call sites) branches on mode: net → existing HTTP module; no-net → the new module.
- Command payload: `"<player>"` name (quoted, handles spaces) for the proxy to match against `session.player.name`; `<audio_id>` (the disc id; the mod does not hold the server `event_id`); block coords; `<dimension>` (mandatory).
- Eject path unchanged.

### Proxy (Rust, no-net only)
- **Remove** the `ItemUse`-sniff insert path (`InventoryTransactionHandler` / `PlayerAuthInputHandler` jukebox-insert emit) and `note_insert_pending`.
- **Add** a handler that reads the intercepted `/bvc:play`, parses `player`/`player_uuid`/`audio_id`/pos/dimension, and **if it matches this session's player**, emits `JukeboxInsert` over QUIC. Suppress (drop) the message if the relay can; otherwise it's visible (accepted).
- **Keep** eject paths: player-eject and the `JukeboxEjectInjector` / track-end relay. Keep the beacon (`pos→event_id`) for eject resolution.

### Server (Rust)
- No changes.

## Open / verify (not blockers)
- **Suppression of `/bvc:play`:** best-effort. The mod's `chatSend.cancel` only suppresses serverbound player chat (that's how `!bvce` hides); a mod-originated `/bvc:play` is server→client, so hiding it requires the proxy to drop the intercepted packet. If the relay can't drop, it's visible — accepted.
- **Wire format:** confirm how the mod best emits the command (`/say`-style system message vs `sendMessage`) and that the proxy can parse it; pick whichever is cleanly interceptable.
- **Fulfiller match:** proxy compares command `player`/`player_uuid` against its session player; deterministic, single fulfiller.

## Verification plan
- `cargo build` (client, no-net path) + `yarn rebuild` (mod) clean.
- No-net manual matrix: click-insert plays; **swap ejects + stops, disc stays in hand**; **hopper insert plays (closest player)**; player eject stops; track-end auto-eject stops + block clears.
- Net: unchanged via HTTP (regression).
