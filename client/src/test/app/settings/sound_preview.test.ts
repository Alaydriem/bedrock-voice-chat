import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { SoundPreview } from "../../../js/app/settings/SoundPreview";

interface FakeAudio {
    src: string;
    readonly plays: number;
    readonly pauses: number;
    end: () => void;
}

const made: FakeAudio[] = [];
let original: unknown;

class Stub {
    src: string;
    plays = 0;
    pauses = 0;
    private ended: (() => void) | null = null;

    constructor(src: string) {
        this.src = src;
        made.push(this as unknown as FakeAudio);
    }
    addEventListener(name: string, handler: () => void): void {
        if (name === "ended") this.ended = handler;
    }
    async play(): Promise<void> {
        this.plays += 1;
    }
    pause(): void {
        this.pauses += 1;
    }
    end(): void {
        this.ended?.();
    }
}

function current(preview: SoundPreview): string | null {
    let value: string | null = null;
    preview.playing.subscribe((v) => (value = v))();
    return value;
}

beforeEach(() => {
    made.length = 0;
    original = globalThis.Audio;
    globalThis.Audio = Stub as unknown as typeof Audio;
});

afterEach(() => {
    globalThis.Audio = original as typeof Audio;
});

describe("SoundPreview", () => {
    it("plays the sound it is given the url for", async () => {
        const preview = new SoundPreview(async (id) => `https://bvc.example.com/${id}`);
        await preview.toggle("snd_airhorn");

        expect(made).toHaveLength(1);
        expect(made[0]?.src).toBe("https://bvc.example.com/snd_airhorn");
        expect(made[0]?.plays).toBe(1);
        expect(current(preview)).toBe("snd_airhorn");
    });

    // Pressing the button of the row already playing is how it is stopped, so the same
    // button has to carry both meanings.
    it("stops the sound when its own row is pressed again", async () => {
        const preview = new SoundPreview(async (id) => id);
        await preview.toggle("snd_airhorn");
        await preview.toggle("snd_airhorn");

        expect(made[0]?.pauses).toBe(1);
        expect(current(preview)).toBeNull();
        expect(made).toHaveLength(1);
    });

    // Two tracks over each other is nobody's intent, and only the second row would have a
    // stop button.
    it("stops the first sound when a second one starts", async () => {
        const preview = new SoundPreview(async (id) => id);
        await preview.toggle("snd_airhorn");
        await preview.toggle("snd_bell");

        expect(made[0]?.pauses).toBe(1);
        expect(current(preview)).toBe("snd_bell");
    });

    // A stop button left lit over a track that already finished offers to stop silence.
    it("clears itself when the sound reaches its end", async () => {
        const preview = new SoundPreview(async (id) => id);
        await preview.toggle("snd_airhorn");
        made[0]?.end();

        expect(current(preview)).toBeNull();
    });

    it("reports nothing playing when the url cannot be had", async () => {
        const preview = new SoundPreview(async () => {
            throw new Error("no token");
        });
        await preview.toggle("snd_airhorn");

        expect(current(preview)).toBeNull();
        expect(made).toHaveLength(0);
    });

    it("is quiet after stop is called with nothing playing", () => {
        const preview = new SoundPreview(async (id) => id);
        expect(() => preview.stop()).not.toThrow();
        expect(current(preview)).toBeNull();
    });
});

