import { system } from '@minecraft/server';
import type { Player } from '@minecraft/server';
import { httpClient } from '../net';
import type { ServerAdminConfig } from '../config';
import type { NoNetControlSender } from '../control/sender';
import type { GroupCache, GroupRow } from './group_cache';

const REQUEST_TIMEOUT_SEC = 2;

// A CustomForm's rows are fixed when it is built, so a list that arrives after the
// page opens cannot appear. The page waits this long for the rides before it builds.
const NO_NET_WAIT_TICKS = 20;

/// Supplies the Groups page with the server's group list. Net mode reads
/// `GET /api/channel`; no-net mode asks the desktop proxy and waits for the ride.
///
/// `null` means the list could not be read. An empty array means the server has no
/// groups. The page says something different for each, so they never collapse.
export interface GroupSource {
  list(player: Player): Promise<GroupRow[] | null>;
  // Asks early, so the page's own `list` resolves at once. Net mode has nothing to
  // warm: its `list` is one request it makes when the page opens.
  warm(player: Player): void;
}

export class NetGroupSource implements GroupSource {
  constructor(
    private readonly config: ServerAdminConfig,
    private readonly cache: GroupCache,
  ) {}

  // Net mode could answer the page without the cache, but the panel's status line
  // resolves a share code to a name through it, so the fetch fills it either way.
  warm(player: Player): void {
    void this.list(player);
  }

  async list(_player: Player): Promise<GroupRow[] | null> {
    const response = await httpClient.request(
      `${this.config.bvcServer}/api/channel`,
      'Get',
      undefined,
      [
        ['Authorization', `Bearer ${this.config.accessToken}`],
        ['Accept', 'application/json'],
      ],
      REQUEST_TIMEOUT_SEC,
    );
    if (!response || response.status < 200 || response.status >= 300) {
      return null;
    }
    const rows = NetGroupSource.parse(response.body);
    if (rows !== null) {
      this.cache.beginList(rows.length);
      for (const row of rows) {
        this.cache.add(row);
      }
    }
    return rows;
  }

  // `Channel` carries players and creator as well; the panel reads neither.
  private static parse(body: string): GroupRow[] | null {
    try {
      const parsed: unknown = JSON.parse(body);
      if (!Array.isArray(parsed)) {
        return null;
      }
      const rows: GroupRow[] = [];
      for (const item of parsed) {
        if (item === null || typeof item !== 'object') {
          continue;
        }
        const c = item as Record<string, unknown>;
        if (typeof c.id === 'string' && typeof c.name === 'string') {
          rows.push({ id: c.id, name: c.name });
        }
      }
      return rows;
    } catch {
      return null;
    }
  }
}

export class NoNetGroupSource implements GroupSource {
  constructor(
    private readonly sender: () => NoNetControlSender | null,
    private readonly cache: GroupCache,
  ) {}

  warm(player: Player): void {
    void this.sender()?.requestGroups(player);
  }

  async list(player: Player): Promise<GroupRow[] | null> {
    const sender = this.sender();
    if (!sender) {
      return null;
    }
    await sender.requestGroups(player);
    const settled = await this.settled();
    if (!settled) {
      // The header promised rows that never all arrived; show the ones that did.
      this.cache.settlePartial();
    }
    return this.cache.answered() ? this.cache.rows() : null;
  }

  // Resolves true on the first complete list after the request, false on the cap.
  private settled(): Promise<boolean> {
    return new Promise((resolve) => {
      let done = false;
      const finish = (complete: boolean): void => {
        if (done) {
          return;
        }
        done = true;
        off();
        resolve(complete);
      };
      const off = this.cache.onChange(() => {
        finish(true);
      });
      system.runTimeout(() => {
        finish(false);
      }, NO_NET_WAIT_TICKS);
    });
  }
}
