import type { GroupRow } from './group_cache';

/// Reads `GET /api/channel` into panel rows. Kept apart from the source so it imports nothing
/// from the game and can be tested on its own.
export class GroupRowParser {
  // `Channel` carries players and creator as well; the panel reads neither.
  static parse(body: string): GroupRow[] | null {
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
          rows.push({
            id: c.id,
            name: c.name,
            protected: c.protected === true,
          });
        }
      }
      return rows;
    } catch {
      return null;
    }
  }
}
