import { writable, type Writable, type Readable } from 'svelte/store';
import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow';
import type { BedrockLogEntry } from '../../../../bindings/BedrockLogEntry';

export class BedrockLogsManager {
    // Info and above. What a reader opened the pane to see, and what a support
    // paste has to contain.
    public static readonly MAX_LOG_ENTRIES = 200;

    // Debug and trace, capped separately. `bedrock_protocol` and `raknet` are
    // captured at debug, so a running proxy produces thousands of these a minute;
    // sharing one budget let them flush every info line out of the buffer within
    // seconds, which read as the log clearing itself while the proxy ran.
    public static readonly MAX_DEBUG_ENTRIES = 200;

    private realmsLogsStore: Writable<BedrockLogEntry[]>;
    private logsExpandedStore: Writable<boolean>;
    private logUnlisten: (() => void) | null = null;
    private destroyed = false;

    public readonly realmsLogs: Readable<BedrockLogEntry[]>;
    public readonly logsExpanded: Readable<boolean>;

    constructor() {
        this.realmsLogsStore = writable([]);
        this.logsExpandedStore = writable(false);
        this.realmsLogs = { subscribe: this.realmsLogsStore.subscribe };
        this.logsExpanded = { subscribe: this.logsExpandedStore.subscribe };
    }

    async initialize(): Promise<void> {
        await this.subscribe();
    }

    private async subscribe(): Promise<void> {
        if (this.logUnlisten || this.destroyed) {
            return;
        }
        const appWebview = getCurrentWebviewWindow();
        const unlisten = await appWebview.listen<BedrockLogEntry>(
            'bedrock-log',
            (event) => {
                const entry = event.payload;

                // Every level is buffered. The view decides what to show, so its
                // debug toggle has something to reveal; dropping levels here made
                // that toggle inert.
                this.realmsLogsStore.update((current) =>
                    BedrockLogsManager.retain([...current, entry]),
                );
            },
        );

        // `listen` is a round trip, and a destroy landing inside it used to leave the
        // listener attached with nothing left to release it — writing into a store nobody
        // reads for the life of the process.
        if (this.destroyed) {
            unlisten();
            return;
        }
        this.logUnlisten = unlisten;
    }

    static isVerbose(entry: BedrockLogEntry): boolean {
        const level = entry.level.toLowerCase();
        return level.startsWith('debug') || level.startsWith('trace');
    }

    /**
     * Trims each class of line against its own budget, oldest first.
     *
     * Two budgets rather than one, so neither class can starve the other: a debug flood
     * cannot evict the info line a reader came for, and a long-running session's info
     * cannot leave the debug toggle with nothing to reveal.
     */
    private static retain(entries: BedrockLogEntry[]): BedrockLogEntry[] {
        let verbose = 0;
        let ordinary = 0;
        for (const entry of entries) {
            if (BedrockLogsManager.isVerbose(entry)) verbose += 1;
            else ordinary += 1;
        }

        let dropVerbose = Math.max(0, verbose - BedrockLogsManager.MAX_DEBUG_ENTRIES);
        let dropOrdinary = Math.max(0, ordinary - BedrockLogsManager.MAX_LOG_ENTRIES);
        if (dropVerbose === 0 && dropOrdinary === 0) {
            return entries;
        }

        const kept: BedrockLogEntry[] = [];
        for (const entry of entries) {
            if (BedrockLogsManager.isVerbose(entry)) {
                if (dropVerbose > 0) {
                    dropVerbose -= 1;
                    continue;
                }
            } else if (dropOrdinary > 0) {
                dropOrdinary -= 1;
                continue;
            }
            kept.push(entry);
        }
        return kept;
    }

    toggleLogs(): void {
        this.logsExpandedStore.update((v) => !v);
    }

    clearLogs(): void {
        this.realmsLogsStore.set([]);
    }

    destroy(): void {
        this.destroyed = true;
        if (this.logUnlisten) {
            this.logUnlisten();
            this.logUnlisten = null;
        }
    }
}
