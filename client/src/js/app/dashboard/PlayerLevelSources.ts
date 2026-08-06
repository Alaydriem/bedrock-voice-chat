import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow';
import type { UnlistenFn } from '@tauri-apps/api/event';
import { warn } from '@tauri-apps/plugin-log';
import type { LevelSource } from '$radial/core/sources/LevelSource';
import { PushLevelSource } from '$radial/core/sources/LevelSource';
import { LevelScale } from '$radial/core/sources/LevelScale';
import GameNameUtils from '../utils/GameNameUtils';

/**
 * One level source per speaker, from the `audio-activity` event.
 *
 * The event carries only the players who produced audio in the last batch, and the map is
 * cleared each time — so a player going quiet is reported by their absence rather than by a
 * zero. A meter left at its last value reads as somebody still talking, which is the one
 * thing it must never say, so silence decays here.
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
    private unlisten: UnlistenFn | null = null;
    private sweep: ReturnType<typeof setInterval> | null = null;

    async start(): Promise<void> {
        this.stop();
        try {
            this.unlisten = await getCurrentWebviewWindow().listen<Record<string, number>>(
                'audio-activity',
                (event) => this.receive(event.payload),
            );
        } catch (e) {
            warn(`PlayerLevelSources: could not listen for activity: ${e}`);
        }

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

    private receive(activity: Record<string, number>): void {
        const now = performance.now();
        for (const [name, level] of Object.entries(activity)) {
            const key = GameNameUtils.canonical(name);
            this.seenAt.set(key, now);
            // Scaled for the same reason your own meter is: a linear RMS spends its whole
            // range on levels nobody produces.
            (this.for(key) as PushLevelSource).push(LevelScale.fromRms(level));
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
    }
}
