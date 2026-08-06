import { type Readable, type Writable, derived, get, writable } from 'svelte/store';
import { warn } from '@tauri-apps/plugin-log';
import type { Store } from '@tauri-apps/plugin-store';
import { PlayerHue } from '$radial/core/sources/PlayerHue';
import type { PlayerGainStore } from '../../bindings/PlayerGainStore';
import type { PositionSnapshot } from '../../bindings/PositionSnapshot';
import type { RelativePosition } from '../../bindings/RelativePosition';
import GameNameUtils from '../utils/GameNameUtils';
import type { NearbyPlayer } from './NearbyPlayer';
import { PositionFeed } from './PositionFeed';

/**
 * Who is near you, and how far away.
 *
 * The feed is the single authority for both. Audio was the obvious candidate and is the
 * wrong one: a client only receives a player's coordinates stamped on that player's own
 * audio frames, so somebody standing next to you in silence has no position anywhere on this
 * machine. They would be invisible until they spoke, and then vanish when they stopped.
 *
 * Membership is a range test rather than a timeout for the same reason. `broadcast_range`
 * comes from the server's own config, so the boundary the UI draws is the boundary the audio
 * router uses — not an approximation of it.
 */
export class NearbyManager {
    /**
     * How long a player survives being absent from snapshots.
     *
     * Snapshots arrive at 2 Hz and each is complete, so absence is meaningful immediately —
     * but a dropped frame or a reconnect must not empty the roster. Fifteen seconds is long
     * enough to ride out a reconnect and short enough that somebody who walked away is gone
     * before you wonder why they are still listed.
     */
    static readonly FALLOFF_MS = 15_000;

    /** Until `/api/config` answers, the kit's own default. */
    private static readonly DEFAULT_RANGE_M = 48;

    private readonly playersStore: Writable<readonly NearbyPlayer[]>;
    private readonly store: Store;
    private feed: PositionFeed | null = null;
    private range = NearbyManager.DEFAULT_RANGE_M;

    /** Last time each player appeared in a snapshot, for the falloff. */
    private seenAt = new Map<string, number>();
    /** The most recent entry per player, so absence can be distinguished from silence. */
    private latest = new Map<string, NearbyPlayer>();
    private sweep: ReturnType<typeof setInterval> | null = null;

    /** Everyone the feed can see, nearest first. */
    public readonly players: Readable<readonly NearbyPlayer[]>;
    /** Those inside voice range — the roster. */
    public readonly inEarshot: Readable<readonly NearbyPlayer[]>;
    /** Those beyond it but within feed scope — the ring's cast. */
    public readonly approaching: Readable<readonly NearbyPlayer[]>;

    constructor(store: Store) {
        this.store = store;
        this.playersStore = writable([]);
        this.players = { subscribe: this.playersStore.subscribe };
        this.inEarshot = derived(this.playersStore, ($all) => $all.filter((p) => p.inEarshot));
        this.approaching = derived(this.playersStore, ($all) => $all.filter((p) => !p.inEarshot));
    }

    /**
     * @param range The server's `broadcast_range`. Passed in rather than read here because
     *   the dashboard already fetches `/api/config` during boot, and asking twice invites the
     *   two answers to disagree.
     */
    async start(server: string, range: number | null): Promise<void> {
        this.stop();
        if (range && range > 0) this.range = range;

        this.feed = new PositionFeed(server, (snapshot) => this.receive(snapshot));
        await this.feed.start();

        this.sweep = setInterval(() => this.expire(), 1_000);
    }

    private receive(snapshot: PositionSnapshot): void {
        const now = Date.now();
        for (const entry of snapshot.positions) {
            const player = this.toPlayer(entry);
            this.latest.set(player.name, player);
            const first = !this.seenAt.has(player.name);
            this.seenAt.set(player.name, now);
            if (first) void this.remember(player);
        }
        this.publish();
    }

    private toPlayer(entry: RelativePosition): NearbyPlayer {
        const game = GameNameUtils.extractGame(entry.name) ?? 'minecraft';
        return {
            name: entry.name,
            gamertag: GameNameUtils.stripPrefix(entry.name),
            game,
            // Lowercased, because the glyph beside this hue is derived from the same key and
            // two derivations of one identity must not disagree about its colour.
            hue: PlayerHue.of(entry.name.toLowerCase()),
            presence: entry.presence,
            distance: entry.distance,
            bearing: (entry.bearing_deg * Math.PI) / 180,
            elevation: entry.elevation,
            inEarshot: entry.distance <= this.range,
        };
    }

    /**
     * Drop anyone who has stopped appearing.
     *
     * Absence is the departure signal, not a distance test: somebody who changed dimension,
     * became a spectator or walked past feed scope stops being reported at all, and each of
     * those is a reason they can no longer hear you.
     */
    private expire(): void {
        const cutoff = Date.now() - NearbyManager.FALLOFF_MS;
        let dropped = false;
        for (const [name, at] of this.seenAt) {
            if (at >= cutoff) continue;
            this.seenAt.delete(name);
            this.latest.delete(name);
            dropped = true;
        }
        if (dropped) this.publish();
    }

    private publish(): void {
        const all = [...this.latest.values()].sort((a, b) => a.distance - b.distance);
        this.playersStore.set(all);
    }

    /**
     * Record a player the first time they are seen.
     *
     * The persisted gain store is already the list of players this device has an opinion
     * about; stamping when they were last seen turns it into the recently-seen list as well,
     * without a second store to keep in step with it.
     */
    private async remember(player: NearbyPlayer): Promise<void> {
        try {
            // `player.name` is already the canonical identity from the position feed;
            // `player.gamertag` is the bare display form and would not resolve at the mixer.
            const gains = ((await this.store.get('player_gain_store')) as PlayerGainStore) || {};
            const existing = gains[player.name] ?? { gain: 1.0, muted: false };
            gains[player.name] = { ...existing, last_seen: Date.now() };
            await this.store.set('player_gain_store', gains);
            await this.store.save();
        } catch (e) {
            warn(`NearbyManager: could not record ${player.gamertag}: ${e}`);
        }
    }

    /** The player entry for a name, if the feed currently reports them. */
    find(name: string): NearbyPlayer | undefined {
        return get(this.playersStore).find((p) => p.name === name);
    }

    stop(): void {
        if (this.sweep) {
            clearInterval(this.sweep);
            this.sweep = null;
        }
        if (this.feed) {
            this.feed.stop();
            this.feed = null;
        }
        this.seenAt.clear();
        this.latest.clear();
        this.playersStore.set([]);
    }
}
