import { invoke } from "@tauri-apps/api/core";
import { info } from "@charlesportwoodii/tauri-plugin-curia";
import { writable, type Readable, type Writable } from "svelte/store";
import FaultCatalog from "../error/FaultCatalog";

export type SessionFlags = Pick<Storage, "getItem" | "setItem">;

/**
 * The update offered at launch, independent of any server.
 *
 * Checked in the background rather than before the first screen, so a slow manifest host
 * delays nothing. A dismissal lasts until the app is next launched: session storage
 * survives the document replacements several screens make, and is cleared when the
 * webview exits.
 */
export class UpdateOffer {
    private static readonly DISMISSED_KEY = "bvc:update-offer-dismissed";

    private readonly offerStore: Writable<string | null>;
    public readonly offer: Readable<string | null>;

    private readonly checkFor: () => Promise<string | null>;
    private readonly session: SessionFlags | null;

    constructor(
        checkFor?: () => Promise<string | null>,
        session: SessionFlags | null = UpdateOffer.sessionStorage(),
    ) {
        this.checkFor = checkFor ?? (() => invoke<string | null>("check_for_updates"));
        this.session = session;
        this.offerStore = writable(null);
        this.offer = { subscribe: this.offerStore.subscribe };
    }

    async check(): Promise<void> {
        if (this.isDismissed()) return;

        try {
            this.offerStore.set(await this.checkFor());
        } catch (e) {
            info(`No update offered: ${e}`);
        }
    }

    /** The install screen for the offered version, or null when nothing is offered. */
    accept(): string | null {
        let version: string | null = null;
        this.offerStore.update((current) => {
            version = current;
            return null;
        });
        return version ? FaultCatalog.updateHref(version) : null;
    }

    dismiss(): void {
        this.offerStore.set(null);
        try {
            this.session?.setItem(UpdateOffer.DISMISSED_KEY, "1");
        } catch {
            // Without storage the dismissal holds only until the next document load.
        }
    }

    private isDismissed(): boolean {
        try {
            return this.session?.getItem(UpdateOffer.DISMISSED_KEY) === "1";
        } catch {
            return false;
        }
    }

    private static sessionStorage(): SessionFlags | null {
        try {
            return window.sessionStorage;
        } catch {
            return null;
        }
    }
}
