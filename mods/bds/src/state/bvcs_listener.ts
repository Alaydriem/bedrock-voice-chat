import { system, world } from '@minecraft/server';
import type { ChatSendBeforeEvent } from '@minecraft/server';
import { BvcsCodec } from './bvcs_codec';
import type { GroupCache } from './group_cache';
import type { StateCache } from './state_cache';

// Parses proxy-injected `!bvcs:` reverse-ride chat into the panel's state cache
// and cancels the message so it never reaches players. The chat sender IS the
// subject: the proxy injects each ride on its own player's session, so the
// state is attributed to `ev.sender` — a player cannot poison another's cache
// without also chatting as them.
export class BvcsListener {
  constructor(
    private readonly cacheFor: (playerName: string) => StateCache,
    // Server-wide, unlike the per-player state caches: every player sees the same
    // groups, so the rides fill one shared list.
    private readonly groups: GroupCache,
  ) {}

  buildHandler(): (ev: ChatSendBeforeEvent) => void {
    return (ev: ChatSendBeforeEvent): void => {
      if (!BvcsCodec.isBvcs(ev.message)) {
        return;
      }
      // Cancelling must happen synchronously inside the before-event; the
      // apply must NOT: before-event handlers run in restricted (read-only)
      // execution where native Observable setData/getData throw. Hop to a
      // normal tick before touching the cache (same pattern as the jukebox
      // chat listeners).
      ev.cancel = true;

      const msg = BvcsCodec.decode(ev.message);
      if (!msg) {
        return;
      }
      const sender = ev.sender.name;

      system.run(() => {
        if (msg.kind === 'gl') {
          this.groups.beginList(msg.count);
          return;
        }
        if (msg.kind === 'g') {
          this.groups.add({ id: msg.id, name: msg.name });
          return;
        }
        const cache = this.cacheFor(sender);
        if (msg.kind === 'q') {
          cache.applyQueryState({
            id: sender,
            muted: msg.muted,
            deafened: msg.deafened,
            recording: msg.recording,
            current_group: msg.group,
          });
        } else {
          cache.applyPreferences([
            {
              owner: sender,
              target: msg.target,
              volume: msg.volume,
              muted: !msg.heard,
            },
          ]);
        }
      });
    };
  }

  register(): void {
    world.beforeEvents.chatSend.subscribe(this.buildHandler());
  }
}
