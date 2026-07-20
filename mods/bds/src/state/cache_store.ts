import { StateCache } from './state_cache';

// Per-player panel state caches, keyed by gamertag. Each player who opens the
// control panel (or whose proxy rides state in) gets their own reactive cache;
// the panel and the feeds (net poll / !bvcs: listener) share instances here.
export class StateCacheStore {
  private readonly caches = new Map<string, StateCache>();

  for(playerName: string): StateCache {
    let cache = this.caches.get(playerName);
    if (!cache) {
      cache = new StateCache();
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
