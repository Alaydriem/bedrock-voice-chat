import type { GroupCache } from './group_cache';
import { StateCache } from './state_cache';

// Per-player panel state caches, keyed by gamertag. Each player who opens the
// control panel (or whose proxy rides state in) gets their own reactive cache;
// the panel and the feeds (net poll / !bvcs: listener) share instances here.
export class StateCacheStore {
  private readonly caches = new Map<string, StateCache>();

  // The group list is server-wide, so it lives here rather than in each cache.
  // A list arriving after a snapshot re-renders every open status line, which is
  // what turns `Group: AB12-CD34` into `Group: Sculk Striders`.
  constructor(private readonly groups: GroupCache) {
    groups.onChange(() => {
      for (const cache of this.caches.values()) {
        cache.refreshStatus();
      }
    });
  }

  for(playerName: string): StateCache {
    let cache = this.caches.get(playerName);
    if (!cache) {
      cache = new StateCache((id) => this.groups.nameFor(id));
      this.caches.set(playerName, cache);
    }
    return cache;
  }

  // A departed player's cache is stale (and any chatter can mint their own
  // entry via !bvcs:); evict on leave so the map tracks the live roster.
  evict(playerName: string): void {
    this.caches.delete(playerName);
  }
}
