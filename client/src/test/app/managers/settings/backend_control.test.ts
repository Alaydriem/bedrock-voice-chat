import { get } from "svelte/store";
import { describe, expect, it, vi } from "vitest";
import "../../../tauri";

const { BackendControl } = await import("../../../../js/app/managers/settings/BackendControl");

interface Deferred<T> {
    readonly promise: Promise<T>;
    resolve(value: T): void;
    reject(reason: unknown): void;
}

function deferred<T>(): Deferred<T> {
    let resolve!: (value: T) => void;
    let reject!: (reason: unknown) => void;
    const promise = new Promise<T>((res, rej) => {
        resolve = res;
        reject = rej;
    });
    return { promise, resolve, reject };
}

describe("a control the backend owns", () => {
    /** The backend clamps, so what it reached is the only honest thing to draw. */
    it("settles on the value the backend applied", async () => {
        const control = new BackendControl<number>(
            100,
            async () => 150,
            async () => 100,
            0,
        );

        await control.request(400);

        expect(get(control.value)).toBe(150);
    });

    /** A slider drag fires on every step; only where it stopped is worth a round trip. */
    it("sends only the last of a burst", async () => {
        const apply = vi.fn(async (value: number) => value);
        const control = new BackendControl<number>(100, apply, async () => 100, 0);

        const first = control.request(10);
        const second = control.request(20);
        await control.request(30);
        await Promise.all([first, second]);

        expect(apply).toHaveBeenCalledTimes(1);
        expect(apply).toHaveBeenCalledWith(30);
        expect(get(control.value)).toBe(30);
    });

    /**
     * Two requests in flight can answer in either order. The older answer landing last would
     * leave the control on a value the backend has already moved past.
     */
    it("never lets an older answer overwrite a newer request", async () => {
        const answers: Deferred<boolean>[] = [];
        const control = new BackendControl<boolean>(
            false,
            () => {
                const answer = deferred<boolean>();
                answers.push(answer);
                return answer.promise;
            },
            async () => false,
            0,
        );

        const first = control.request(true);
        await vi.waitFor(() => expect(answers).toHaveLength(1));
        const second = control.request(false);
        await vi.waitFor(() => expect(answers).toHaveLength(2));

        answers[1].resolve(false);
        await second;
        answers[0].resolve(true);
        await first;

        expect(get(control.value)).toBe(false);
    });

    /** Nothing was applied, so the control goes back to what the backend actually holds. */
    it("returns to the saved value when the request fails", async () => {
        const control = new BackendControl<boolean>(
            false,
            async () => {
                throw new Error("app state is not ready");
            },
            async () => false,
            0,
        );

        await control.request(true);

        expect(get(control.value)).toBe(false);
    });

    /** Moves under the finger, before any round trip. */
    it("shows the requested value at once", () => {
        const control = new BackendControl<boolean>(
            false,
            () => new Promise<boolean>(() => {}),
            async () => false,
            0,
        );

        void control.request(true);

        expect(get(control.value)).toBe(true);
    });
});
