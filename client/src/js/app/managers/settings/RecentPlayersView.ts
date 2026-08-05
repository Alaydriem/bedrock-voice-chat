import type { Store } from '@tauri-apps/plugin-store';
import { warn } from '@tauri-apps/plugin-log';
import type { PlayerGainStore } from '../../../bindings/PlayerGainStore';

/** One player you have been around, with whatever you decided about them. */
export interface RecentPlayer {
    gamertag: string;
    gain: number;
    muted: boolean;
    /** Unix milliseconds, or null for an entry written before it was recorded. */
    lastSeen: number | null;
}

/**
 * Players this device has been near, newest first.
 *
 * Read from the persisted gain store rather than a list of its own. That store already holds
 * exactly the players a device has an opinion about — its keys are why a volume you set
 * survives a reconnect — so stamping when each was last seen makes it answer this question
 * too, with no second list that can fall out of step with it.
 */
export class RecentPlayersView {
    private readonly store: Store;

    constructor(store: Store) {
        this.store = store;
    }

    async load(): Promise<readonly RecentPlayer[]> {
        try {
            const gains = ((await this.store.get('player_gain_store')) as PlayerGainStore) || {};
            return RecentPlayersView.sort(gains);
        } catch (e) {
            warn(`RecentPlayersView: could not read the gain store: ${e}`);
            return [];
        }
    }

    /**
     * Newest first, with never-stamped entries last.
     *
     * Those are entries written before `last_seen` existed, or by an in-game volume command
     * for somebody who was never nearby. They belong on the list — a volume you set is worth
     * finding again — but not at the top of it, where they would claim to be recent.
     */
    static sort(gains: PlayerGainStore): readonly RecentPlayer[] {
        return Object.entries(gains)
            .map(([gamertag, settings]) => ({
                gamertag,
                gain: settings?.gain ?? 1,
                muted: settings?.muted ?? false,
                lastSeen: settings?.last_seen ?? null,
            }))
            .sort((a, b) => (b.lastSeen ?? 0) - (a.lastSeen ?? 0));
    }

    async setGain(gamertag: string, gain: number): Promise<void> {
        await this.update(gamertag, { gain });
    }

    async setMuted(gamertag: string, muted: boolean): Promise<void> {
        await this.update(gamertag, { muted });
    }

    /**
     * Forget one player's settings entirely.
     *
     * Removing the entry rather than resetting it to defaults, because an entry that exists
     * is what keeps them on this list — and "stop showing me this person" is the thing being
     * asked for.
     */
    async forget(gamertag: string): Promise<void> {
        try {
            const gains = ((await this.store.get('player_gain_store')) as PlayerGainStore) || {};
            delete gains[gamertag];
            await this.store.set('player_gain_store', gains);
            await this.store.save();
        } catch (e) {
            warn(`RecentPlayersView: could not forget ${gamertag}: ${e}`);
        }
    }

    private async update(
        gamertag: string,
        changes: { gain?: number; muted?: boolean },
    ): Promise<void> {
        try {
            const gains = ((await this.store.get('player_gain_store')) as PlayerGainStore) || {};
            const existing = gains[gamertag] ?? { gain: 1.0, muted: false };
            gains[gamertag] = { ...existing, ...changes };
            await this.store.set('player_gain_store', gains);
            await this.store.save();
        } catch (e) {
            warn(`RecentPlayersView: could not update ${gamertag}: ${e}`);
        }
    }
}
