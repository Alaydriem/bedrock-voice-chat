import { LevelFeed } from './LevelFeed';
import type { LevelSource } from '$radial/core/sources/LevelSource';
import { PushLevelSource } from '$radial/core/sources/LevelSource';
import type { LevelSnapshot } from '../../bindings/LevelSnapshot';
import { LevelSteps } from './LevelSteps';
import GameNameUtils from '../utils/GameNameUtils';

/** Whether levels are reaching this window, and what your own last measured. */
export interface MicActivity {
    /** Whether the underlying subscription is registered, end to end. */
    readonly attached: boolean;
    /**
     * Whether this fan-out is still asking the feed for levels.
     *
     * Split from `attached` because the two failures are fixed in different places: a feed that
     * lost its listener is re-opened, and a fan-out that stopped subscribing has to be started.
     * Re-opening does nothing for the second — the feed declines to register for an empty
     * audience — so a readout that cannot tell them apart sends the reader to the wrong one.
     */
    readonly sinkHeld: boolean;
    readonly events: number;
    /** Snapshots that arrived and could not be handled. Never the same fault as none arriving. */
    readonly failures: number;
    readonly eventsPerSecond: number;
    /** Your own level, 0 to 1. */
    readonly lastRms: number;
    /** Milliseconds since the last snapshot, or null if none has ever arrived. */
    readonly silentForMs: number | null;
    /**
     * How many meters are listening to the source this pushes your own level into.
     *
     * Zero while a pill is mounted means the pill is holding a different source object — one
     * nothing writes to — and every other figure here stays healthy while its meter sits flat.
     */
    readonly ownListeners: number;
}

/**
 * One level source per speaker, from the shared `audio-levels` event.
 *
 * Your own is one of them. It used to be a second mechanism entirely — its own subscription,
 * its own push source, owned by the controller and rebuilt whenever a reconnect replaced it —
 * and the two did not behave the same: the roster's meters moved and the pill's did not.
 * Nothing about `own` justifies a separate path; it is another entry in the same snapshot, so
 * it is served from the same object, with the same lifetime and the same subscription.
 *
 * The event carries everyone the backend currently knows about, so a player who went quiet is
 * reported as not speaking rather than by absence — but absence still has to mean silence,
 * because a peer who leaves stops appearing at all. A meter left at its last value reads as
 * somebody still talking, which is the one thing it must never say, so silence decays here.
 *
 * Sources are created on demand and kept: a card that is remounted while its player is still
 * around should pick up the same source rather than start from nothing, and a handful of
 * closures per session is cheaper than reference counting them.
 */
export class PlayerLevelSources {
    /** No activity for this long means they stopped talking. */
    private static readonly SILENCE_MS = 300;

    private static readonly SWEEP_MS = 100;

    private readonly sources = new Map<string, PushLevelSource>();
    private readonly seenAt = new Map<string, number>();
    private readonly ownSource = new PushLevelSource();
    private unlisten: (() => void) | null = null;
    private sweep: ReturnType<typeof setInterval> | null = null;
    private received = 0;
    private failures = 0;
    private lastOwn = 0;
    private lastPush = 0;
    private startedAt = 0;

    async start(): Promise<void> {
        this.stop();
        this.received = 0;
        this.failures = 0;
        this.startedAt = performance.now();
        this.unlisten = LevelFeed.shared().subscribe(
            (snapshot) => this.receive(snapshot),
            'PlayerLevelSources',
        );

        this.sweep = setInterval(() => this.decay(), PlayerLevelSources.SWEEP_MS);
    }

    /**
     * The source for a player, by any name form.
     *
     * Keyed on the canonical identity, which is what the audio pipeline reports and what the
     * roster holds. Composed here anyway, so a caller holding a bare name still finds the one
     * source for that player rather than opening a second one that nothing ever pushes to.
     */
    for(name: string): LevelSource {
        const key = GameNameUtils.canonical(name);
        let source = this.sources.get(key);
        if (!source) {
            source = new PushLevelSource();
            this.sources.set(key, source);
        }
        return source;
    }

    /** Your own microphone, for the pill. The same object for the life of this instance. */
    own(): LevelSource {
        return this.ownSource;
    }

    /**
     * Proof of life for the level feed, for the diagnostics readout.
     *
     * Counted here rather than in a second reader, so the number the panel prints is the one
     * belonging to the object that actually drives the meters. A count kept somewhere else can
     * disagree with them, and did.
     */
    get activity(): MicActivity {
        const elapsed = this.startedAt ? (performance.now() - this.startedAt) / 1000 : 0;
        return {
            attached: this.unlisten !== null && LevelFeed.shared().attached,
            sinkHeld: this.unlisten !== null,
            events: this.received,
            failures: this.failures,
            eventsPerSecond: elapsed > 0 ? this.received / elapsed : 0,
            lastRms: this.lastOwn,
            silentForMs: this.lastPush ? performance.now() - this.lastPush : null,
            ownListeners: this.ownSource.listeners,
        };
    }

    private receive(snapshot: LevelSnapshot): void {
        const now = performance.now();
        const present = new Set<string>();

        // Counted before any work, so a snapshot that could not be handled is never mistaken
        // for one that never came.
        this.received += 1;
        this.lastPush = now;
        try {
            this.lastOwn = LevelSteps.toLevel(snapshot.own);
            this.ownSource.push(this.lastOwn);
        } catch {
            this.failures += 1;
        }

        for (const [name, level] of Object.entries(snapshot.peers)) {
            const key = GameNameUtils.canonical(name);
            present.add(key);
            if (!level.speaking) continue;
            this.seenAt.set(key, now);
            (this.for(key) as PushLevelSource).push(LevelSteps.toLevel(level));
        }

        // Anyone the backend no longer lists has gone, and their meter has to be told. The
        // decay below would get there eventually; doing it here stops a departed player's card
        // holding a level for the length of the silence window.
        for (const [key, source] of this.sources) {
            if (!present.has(key) && source.level !== 0) source.push(0);
        }
    }

    private decay(): void {
        const cutoff = performance.now() - PlayerLevelSources.SILENCE_MS;
        for (const [key, source] of this.sources) {
            if (source.level === 0) continue;
            const at = this.seenAt.get(key) ?? 0;
            if (at >= cutoff) continue;
            source.push(0);
        }
    }

    stop(): void {
        if (this.unlisten) {
            this.unlisten();
            this.unlisten = null;
        }
        if (this.sweep) {
            clearInterval(this.sweep);
            this.sweep = null;
        }
        for (const source of this.sources.values()) source.push(0);
        this.ownSource.push(0);
    }
}
