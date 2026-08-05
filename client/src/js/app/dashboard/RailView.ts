import type { ServerListEntry } from '../../bindings/ServerListEntry';
import { ServerRosterManager } from '../server/ServerRosterManager';

/** One button on the rail. */
export interface RailServer {
    /** The stored url, which is what switching servers is keyed on. */
    server: string;
    /** Hostname, and the key the glyph is derived from. */
    host: string;
    /** Who you are signed in as there, for the tooltip. */
    player: string;
    isCurrent: boolean;
}

/**
 * The saved servers, as the rail sees them.
 *
 * Deliberately not `ServerRosterManager`: that runs a four-check preflight per server
 * because picking one is a decision, and a rail is not a decision — it is where you
 * already are plus the others you could switch to. Running preflights to draw it would
 * spend four round trips per server on a screen that shows a 36-pixel glyph.
 */
export class RailView {
    static rows(saved: readonly ServerListEntry[], current: string): readonly RailServer[] {
        return saved.map((entry) => ({
            server: entry.server,
            host: ServerRosterManager.hostOf(entry.server),
            player: entry.player,
            isCurrent: entry.server === current,
        }));
    }
}
