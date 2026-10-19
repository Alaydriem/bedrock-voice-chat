import { I18n } from "#lib/i18n/index.js";
import { type Readable, type Writable, writable } from "svelte/store";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import { error as logError } from "@charlesportwoodii/tauri-plugin-curia";
import type { ProtectedGroupEpochNotice } from "../../bindings/ProtectedGroupEpochNotice";
import type { ProtectedGroupLostNotice } from "../../bindings/ProtectedGroupLostNotice";
import type { ProtectedGroupRosterNotice } from "../../bindings/ProtectedGroupRosterNotice";

interface GroupData {
    in_group: boolean;
    id: string | null;
    name: string | null;
}

/**
 * The webview's view of this client's end-to-end encrypted group.
 *
 * Everything here comes from the app's Rust side, which holds the group's keys: the roster is the
 * group's own MLS state rather than the server's channel list, and nothing that could decrypt the
 * group ever reaches this page. The password passes through once, as a command argument.
 */
export class ProtectedGroupManager {
    static readonly JOINED = "protected-group:joined";
    static readonly ROSTER = "protected-group:roster";
    static readonly EPOCH = "protected-group:epoch";
    static readonly LOST = "protected-group:lost";

    private readonly rosterStore: Writable<string[]>;
    private readonly epochStore: Writable<number | null>;
    private readonly safetyCodeStore: Writable<string | null>;
    private readonly lostReasonStore: Writable<string | null>;
    private readonly activeStore: Writable<boolean>;
    private unlisteners: (() => void)[] = [];

    public readonly roster: Readable<string[]>;
    public readonly epoch: Readable<number | null>;
    public readonly safetyCode: Readable<string | null>;
    public readonly lostReason: Readable<string | null>;
    public readonly active: Readable<boolean>;

    constructor() {
        this.rosterStore = writable([]);
        this.epochStore = writable(null);
        this.safetyCodeStore = writable(null);
        this.lostReasonStore = writable(null);
        this.activeStore = writable(false);

        this.roster = { subscribe: this.rosterStore.subscribe };
        this.epoch = { subscribe: this.epochStore.subscribe };
        this.safetyCode = { subscribe: this.safetyCodeStore.subscribe };
        this.lostReason = { subscribe: this.lostReasonStore.subscribe };
        this.active = { subscribe: this.activeStore.subscribe };
    }

    /** The copy for a lost reason. Every string is a literal so the catalog extractor finds it. */
    static lostMessage(reason: string): string {
        switch (reason) {
            case "wrong_password":
                return I18n.t("Wrong password.");
            case "no_admitter":
                return I18n.t("No member is online to admit you.");
            case "admitter_unreachable":
                return I18n.t("Could not reach a group member.");
            default:
                return I18n.t("Connection to the group was lost. Rejoin to continue.");
        }
    }

    async startListening(): Promise<void> {
        const view = getCurrentWebviewWindow();
        this.unlisteners = await Promise.all([
            view.listen<ProtectedGroupEpochNotice>(ProtectedGroupManager.JOINED, (event) => {
                this.lostReasonStore.set(null);
                this.activeStore.set(true);
                this.applyEpoch(event.payload);
            }),
            view.listen<ProtectedGroupEpochNotice>(ProtectedGroupManager.EPOCH, (event) =>
                this.applyEpoch(event.payload),
            ),
            view.listen<ProtectedGroupRosterNotice>(ProtectedGroupManager.ROSTER, (event) =>
                this.rosterStore.set([...event.payload.members]),
            ),
            view.listen<ProtectedGroupLostNotice>(ProtectedGroupManager.LOST, (event) => {
                this.reset();
                this.lostReasonStore.set(event.payload.reason);
            }),
        ]);
    }

    /** Creates a protected group and joins it. Returns its id, or null on failure. */
    async create(name: string, password: string): Promise<string | null> {
        try {
            this.lostReasonStore.set(null);
            const group = await invoke<GroupData>("group_create_protected", { name, password });
            return group.id;
        } catch (e) {
            logError(`ProtectedGroupManager: create failed: ${e}`);
            return null;
        }
    }

    /** Starts joining. Admission finishes later, as a joined or lost event. */
    async join(id: string, password: string): Promise<boolean> {
        try {
            this.lostReasonStore.set(null);
            await invoke<GroupData>("group_join_protected", { id, password });
            return true;
        } catch (e) {
            logError(`ProtectedGroupManager: join failed: ${e}`);
            return false;
        }
    }

    /**
     * Wipes this client's keys for the group, ahead of leaving its channel. The microphone stays
     * silent until `release`, which the caller sends only once the channel is left.
     */
    async stop(): Promise<void> {
        try {
            await invoke("group_stop_protected");
        } catch (e) {
            logError(`ProtectedGroupManager: stop failed: ${e}`);
        }
        this.reset();
    }

    /** Lets audio flow unsealed again, after the protected channel has been left. */
    async release(): Promise<void> {
        try {
            await invoke("group_release_protected");
        } catch (e) {
            logError(`ProtectedGroupManager: release failed: ${e}`);
        }
    }

    cleanup(): void {
        for (const unlisten of this.unlisteners) unlisten();
        this.unlisteners = [];
    }

    private applyEpoch(notice: ProtectedGroupEpochNotice): void {
        this.epochStore.set(notice.epoch);
        this.safetyCodeStore.set(notice.safety_code);
    }

    private reset(): void {
        this.activeStore.set(false);
        this.rosterStore.set([]);
        this.epochStore.set(null);
        this.safetyCodeStore.set(null);
    }
}
