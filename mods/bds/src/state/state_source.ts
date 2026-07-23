import { system } from '@minecraft/server';
import type { Player } from '@minecraft/server';
import { httpClient } from '../net';
import type { ServerAdminConfig } from '../config';
import type { NoNetControlSender } from '../control/sender';
import type { StateCache, PlayerPreference, QueryState } from './state_cache';

// ~1s at 20 ticks/s.
const NET_POLL_TICKS = 20;
const REQUEST_TIMEOUT_SEC = 1;

/// Feeds a player's StateCache while their control panel is open. Net mode
/// polls the BVC server's control cache; no-net mode asks the desktop proxy for
/// a snapshot ride (live changes already ride in via the BvcsListener).
export interface PanelFeed {
  // `targets` supplies the players the panel is currently displaying, so
  // preference fetches stay scoped and never pull the whole store.
  start(player: Player, cache: StateCache, targets: () => string[]): void;
  // Keyed by name so a disconnect (no live Player handle) can also stop it.
  stop(playerName: string): void;
  // Whether this feed delivers a trustworthy current_group. Net mode reads the
  // server-authoritative overlay from /api/state; the no-net reverse ride
  // cannot (the desktop reporter has no channel state and always sends
  // current_group=null), so the panel must not gate Leave on it there.
  tracksCurrentGroup(): boolean;
}

// Net mode: poll GET /api/state and the scoped GET /api/preferences while the
// panel is open, folding both into the cache.
export class NetStateSource implements PanelFeed {
  private readonly polls = new Map<string, number>();

  constructor(private readonly config: ServerAdminConfig) {}

  start(player: Player, cache: StateCache, targets: () => string[]): void {
    this.stop(player.name);
    const name = player.name;
    const handle = system.runInterval(() => {
      void this.poll(name, cache, targets());
    }, NET_POLL_TICKS);
    this.polls.set(name, handle);
    // Seed immediately rather than waiting a full interval.
    void this.poll(name, cache, targets());
  }

  stop(playerName: string): void {
    const handle = this.polls.get(playerName);
    if (handle !== undefined) {
      system.clearRun(handle);
      this.polls.delete(playerName);
    }
  }

  private async poll(
    name: string,
    cache: StateCache,
    targets: string[],
  ): Promise<void> {
    const headers: Array<[string, string]> = [
      ['X-MC-Access-Token', this.config.accessToken],
      ['Accept', 'application/json'],
    ];

    const stateResponse = await httpClient.request(
      `${this.config.bvcServer}/api/state?id=${encodeURIComponent(name)}`,
      'Get',
      undefined,
      headers,
      REQUEST_TIMEOUT_SEC,
    );
    if (
      stateResponse &&
      stateResponse.status >= 200 &&
      stateResponse.status < 300
    ) {
      const state = this.parseState(stateResponse.body);
      if (state) {
        cache.applyQueryState(state);
      }
    }

    if (targets.length === 0) {
      return;
    }
    const prefsResponse = await httpClient.request(
      `${this.config.bvcServer}/api/preferences?owner=${encodeURIComponent(name)}&targets=${encodeURIComponent(targets.join(','))}`,
      'Get',
      undefined,
      headers,
      REQUEST_TIMEOUT_SEC,
    );
    if (
      prefsResponse &&
      prefsResponse.status >= 200 &&
      prefsResponse.status < 300
    ) {
      cache.applyPreferences(this.parsePreferences(prefsResponse.body));
    }
  }

  tracksCurrentGroup(): boolean {
    return true;
  }

  private parseState(body: string): QueryState | null {
    try {
      const parsed: unknown = JSON.parse(body);
      if (parsed === null || typeof parsed !== 'object') {
        return null;
      }
      const s = parsed as Record<string, unknown>;
      if (typeof s.id !== 'string' || typeof s.muted !== 'boolean') {
        return null;
      }
      return {
        id: s.id,
        muted: s.muted === true,
        deafened: s.deafened === true,
        recording: s.recording === true,
        current_group:
          typeof s.current_group === 'string' ? s.current_group : null,
      };
    } catch {
      return null;
    }
  }

  private parsePreferences(body: string): PlayerPreference[] {
    try {
      const parsed: unknown = JSON.parse(body);
      if (!Array.isArray(parsed)) {
        return [];
      }
      const prefs: PlayerPreference[] = [];
      for (const item of parsed) {
        if (item === null || typeof item !== 'object') {
          continue;
        }
        const p = item as Record<string, unknown>;
        if (
          typeof p.owner === 'string' &&
          typeof p.target === 'string' &&
          typeof p.volume === 'number' &&
          typeof p.muted === 'boolean'
        ) {
          prefs.push({
            owner: p.owner,
            target: p.target,
            volume: p.volume,
            muted: p.muted,
          });
        }
      }
      return prefs;
    } catch {
      return [];
    }
  }
}

// No-net mode: ask the desktop proxy for a scoped snapshot ride on open; the
// BvcsListener keeps the cache live from there (the proxy rides every change).
export class NoNetStateSource implements PanelFeed {
  constructor(private readonly sender: () => NoNetControlSender | null) {}

  start(player: Player, _cache: StateCache, targets: () => string[]): void {
    const sender = this.sender();
    if (sender) {
      void sender.requestSync(targets(), player);
    }
  }

  stop(_playerName: string): void {}

  tracksCurrentGroup(): boolean {
    return false;
  }
}
