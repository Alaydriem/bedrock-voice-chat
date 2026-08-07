import { writable, type Readable, type Writable } from "svelte/store";
import { invoke } from "@tauri-apps/api/core";
import { error } from "@tauri-apps/plugin-log";
import { Store } from "@tauri-apps/plugin-store";
import Analytics from "../../analytics";
import PlatformDetector from "../../utils/PlatformDetector";
import type { WebSocketConfig } from "./WebSocketConfig";
import { AppStore } from "../../services/AppStore";

export class WebSocketSettingsManager {
    private readonly platformDetector = new PlatformDetector();

    private isReadyStore: Writable<boolean>;
    public readonly isReady: Readable<boolean>;
    private isMobileStore: Writable<boolean>;
    public readonly isMobile: Readable<boolean>;
    private localhostOnlyStore: Writable<boolean>;
    public readonly localhostOnly: Readable<boolean>;
    private websocketPortStore: Writable<string>;
    public readonly websocketPort: Readable<string>;
    private authKeyStore: Writable<string>;
    public readonly authKey: Readable<string>;
    private isRunningStore: Writable<boolean>;
    public readonly isRunning: Readable<boolean>;
    private startErrorStore: Writable<string>;
    public readonly startError: Readable<string>;

    private store: Store | null = null;

    constructor() {
        this.isReadyStore = writable(false);
        this.isReady = { subscribe: this.isReadyStore.subscribe };
        this.isMobileStore = writable(false);
        this.isMobile = { subscribe: this.isMobileStore.subscribe };
        this.localhostOnlyStore = writable(true);
        this.localhostOnly = { subscribe: this.localhostOnlyStore.subscribe };
        this.websocketPortStore = writable("9595");
        this.websocketPort = { subscribe: this.websocketPortStore.subscribe };
        this.authKeyStore = writable("");
        this.authKey = { subscribe: this.authKeyStore.subscribe };
        this.isRunningStore = writable(false);
        this.isRunning = { subscribe: this.isRunningStore.subscribe };
        this.startErrorStore = writable("");
        this.startError = { subscribe: this.startErrorStore.subscribe };
    }

    async initialize(): Promise<void> {
        this.store = await AppStore.load();

        const mobile = await this.platformDetector.checkMobile().catch(() => false);
        this.isMobileStore.set(mobile);

        const config = await this.store.get<WebSocketConfig>("websocket_server");
        if (config) {
            // Forced on a phone, not defaulted: an older config would keep a useless bind.
            this.localhostOnlyStore.set(mobile ? false : (config.localhost_only ?? true));
            this.websocketPortStore.set(config.port?.toString() || "9595");
            this.authKeyStore.set(config.key || "");
        } else {
            this.localhostOnlyStore.set(!mobile);
        }

        let running = false;
        try {
            running = await invoke<boolean>("is_websocket_running");
        } catch (e) {
            error(`Failed to check WebSocket server status: ${e}`);
        }
        this.isRunningStore.set(running);

        // Nothing else starts it after a restart.
        if (config?.enabled && !running) {
            await this.startServer();
        }

        this.isReadyStore.set(true);
    }

    async handleLocalhostToggle(): Promise<void> {
        this.localhostOnlyStore.update((value) => !value);
        await this.saveConfig(this.currentRunning());
        await this.restartServerIfRunning();
    }

    async handlePortChange(value: string): Promise<void> {
        this.websocketPortStore.set(value);
        await this.saveConfig(this.currentRunning());
        await this.restartServerIfRunning();
    }

    async handleKeyChange(value: string): Promise<void> {
        this.authKeyStore.set(value);
        await this.saveConfig(this.currentRunning());
        await this.restartServerIfRunning();
    }

    async handleGenerateKey(): Promise<void> {
        try {
            this.authKeyStore.set(await invoke<string>("generate_encryption_key"));
            await this.saveConfig(this.currentRunning());
            await this.restartServerIfRunning();
        } catch (e) {
            error(`Failed to generate encryption key: ${e}`);
        }
    }

    async handleToggleServer(): Promise<void> {
        if (this.currentRunning()) {
            await this.stopServer();
        } else {
            await this.startServer();
        }
    }

    private currentRunning(): boolean {
        let running = false;
        this.isRunningStore.update((value) => {
            running = value;
            return value;
        });
        return running;
    }

    private async saveConfig(enabled: boolean): Promise<void> {
        const config: WebSocketConfig = {
            enabled,
            localhost_only: this.currentLocalhostOnly(),
            port: parseInt(this.currentPort()),
            key: this.currentKey(),
        };
        await this.store?.set("websocket_server", config);
        await this.store?.save();

        await invoke("update_websocket_config", { config });
    }

    private currentLocalhostOnly(): boolean {
        let value = true;
        this.localhostOnlyStore.update((v) => {
            value = v;
            return v;
        });
        return value;
    }

    private currentPort(): string {
        let value = "9595";
        this.websocketPortStore.update((v) => {
            value = v;
            return v;
        });
        return value;
    }

    private currentKey(): string {
        let value = "";
        this.authKeyStore.update((v) => {
            value = v;
            return v;
        });
        return value;
    }

    private async restartServerIfRunning(): Promise<void> {
        if (!this.currentRunning()) return;
        try {
            await invoke("stop_websocket_server");
            await invoke("start_websocket_server");
        } catch (e) {
            error(`Failed to restart WebSocket server: ${e}`);
            this.isRunningStore.set(false);
        }
    }

    private async startServer(): Promise<void> {
        this.startErrorStore.set("");
        try {
            // The backend refuses to start without one.
            if (!this.currentKey().trim()) {
                this.authKeyStore.set(await invoke<string>("generate_encryption_key"));
            }
            await this.saveConfig(true);
            await invoke("start_websocket_server");
            this.isRunningStore.set(true);
            Analytics.track("WebsocketServerToggled", { enabled: 1 });
        } catch (e) {
            error(`Failed to start WebSocket server: ${e}`);
            // The toggle must not sit on for a server that never bound.
            this.isRunningStore.set(false);
            this.startErrorStore.set(e instanceof Error ? e.message : String(e));
        }
    }

    private async stopServer(): Promise<void> {
        try {
            await invoke("stop_websocket_server");
            this.isRunningStore.set(false);
            await this.saveConfig(false);
            Analytics.track("WebsocketServerToggled", { enabled: 0 });
        } catch (e) {
            error(`Failed to stop WebSocket server: ${e}`);
        }
    }
}
