import { writable, type Writable, type Readable } from 'svelte/store';
import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow';
import type { BedrockLogEntry } from '../../../../bindings/BedrockLogEntry';

export class BedrockLogsManager {
    private static readonly MAX_LOG_ENTRIES = 200;
    private static readonly ALLOWED_LOG_LEVELS = new Set(['INFO', 'WARN', 'ERROR']);

    private realmsLogsStore: Writable<BedrockLogEntry[]>;
    private logsExpandedStore: Writable<boolean>;
    private logUnlisten: (() => void) | null = null;

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
        if (this.logUnlisten) {
            return;
        }
        const appWebview = getCurrentWebviewWindow();
        this.logUnlisten = await appWebview.listen<BedrockLogEntry>(
            'bedrock-log',
            (event) => {
                const entry = event.payload;
                if (!BedrockLogsManager.ALLOWED_LOG_LEVELS.has(entry.level)) {
                    return;
                }
                this.realmsLogsStore.update((current) =>
                    [...current, entry].slice(-BedrockLogsManager.MAX_LOG_ENTRIES),
                );
            },
        );
    }

    toggleLogs(): void {
        this.logsExpandedStore.update((v) => !v);
    }

    clearLogs(): void {
        this.realmsLogsStore.set([]);
    }

    destroy(): void {
        if (this.logUnlisten) {
            this.logUnlisten();
            this.logUnlisten = null;
        }
    }
}
