import { invoke } from "@tauri-apps/api/core";
import { derived, writable, type Readable, type Writable } from "svelte/store";

export type UpdateKind = "idle" | "checking" | "current" | "available" | "unavailable" | "failed";

export interface UpdateState {
    readonly kind: UpdateKind;
    /** The version waiting to be installed, when there is one. */
    readonly version: string | null;
    /** Epoch milliseconds of the last completed check, so the row can date itself. */
    readonly checkedAt: number | null;
}

/**
 * Whether a newer build is waiting, from `check_for_updates`.
 *
 * `unavailable` is a build with no updater — a store install — and is not a failure.
 * The error message is the only signal that distinguishes the two.
 */
export class UpdateStatus {
    private readonly stateStore: Writable<UpdateState>;
    public readonly state: Readable<UpdateState>;
    public readonly badge: Readable<boolean>;

    private readonly checkFor: () => Promise<string | null>;
    private inFlight = false;

    constructor(checkFor?: () => Promise<string | null>) {
        this.checkFor = checkFor ?? (() => invoke<string | null>("check_for_updates"));
        this.stateStore = writable({ kind: "idle", version: null, checkedAt: null });
        this.state = { subscribe: this.stateStore.subscribe };
        this.badge = derived(this.stateStore, ($state) => $state.kind === "available");
    }

    async check(): Promise<void> {
        if (this.inFlight) return;
        this.inFlight = true;
        this.stateStore.update((state) => ({ ...state, kind: "checking" }));

        try {
            const version = await this.checkFor();
            this.stateStore.set({
                kind: version ? "available" : "current",
                version,
                checkedAt: Date.now(),
            });
        } catch (e) {
            this.stateStore.set({
                kind: UpdateStatus.absent(e) ? "unavailable" : "failed",
                version: null,
                checkedAt: Date.now(),
            });
        } finally {
            this.inFlight = false;
        }
    }

    private static absent(error: unknown): boolean {
        const message = error instanceof Error ? error.message : String(error);
        return message.toLowerCase().includes("no updater");
    }
}
