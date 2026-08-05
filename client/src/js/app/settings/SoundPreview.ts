import { writable, type Readable, type Writable } from "svelte/store";

/** How a sound is fetched for preview. Supplied so a test needs no server. */
export type StreamUrl = (id: string) => Promise<string>;

/**
 * The local preview of one library sound.
 *
 * One sound plays at a time. Pressing a playing row stops it, and anything that replaces
 * the rows underneath stops it too — a track playing over a table it is no longer part of
 * has no button left to stop it with.
 */
export class SoundPreview {
    private readonly streamUrl: StreamUrl;
    private readonly playingStore: Writable<string | null>;
    private element: HTMLAudioElement | null = null;

    /** The id being played, or null. */
    public readonly playing: Readable<string | null>;

    constructor(streamUrl: StreamUrl) {
        this.streamUrl = streamUrl;
        this.playingStore = writable(null);
        this.playing = { subscribe: this.playingStore.subscribe };
    }

    async toggle(id: string): Promise<void> {
        if (this.isPlaying(id)) {
            this.stop();
            return;
        }
        this.stop();
        try {
            const element = new Audio(await this.streamUrl(id));
            element.addEventListener("ended", () => {
                if (this.isPlaying(id)) this.stop();
            });
            this.element = element;
            this.playingStore.set(id);
            await element.play();
        } catch {
            // A preview that will not play is not a failure of settings.
            this.stop();
        }
    }

    stop(): void {
        if (this.element) {
            this.element.pause();
            this.element.src = "";
            this.element = null;
        }
        this.playingStore.set(null);
    }

    private isPlaying(id: string): boolean {
        let current: string | null = null;
        this.playingStore.subscribe((v) => (current = v))();
        return current === id;
    }
}
