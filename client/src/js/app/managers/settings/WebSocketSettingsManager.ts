import { writable, type Readable, type Writable } from "svelte/store";
import { invoke } from "@tauri-apps/api/core";
import { error } from "@tauri-apps/plugin-log";
import { Store } from "@tauri-apps/plugin-store";
import Analytics from "../../analytics";
import PlatformDetector from "../../utils/PlatformDetector";
import type { WebSocketConfig } from "./WebSocketConfig";
import { AppStore } from "../../services/AppStore";

/**
 * The operator-facing WebSocket server's settings.
 *
 * The server has no enable switch. It is bound for the life of the process, on loopback unless
 * the user asks for more, because the only thing an enable switch bought was a listener that
 * had to be turned on before a Stream Deck could reach it — and a port on 127.0.0.1 behind a
 * token is not the exposure the switch was guarding against. What the switch really controlled
 * was reach, and that is what the toggle now says.
 */
export class WebSocketSettingsManager {
    private readonly platformDetector = new PlatformDetector();

    private isReadyStore: Writable<boolean>;
    public readonly isReady: Readable<boolean>;
    private isMobileStore: Writable<boolean>;
    public readonly isMobile: Readable<boolean>;
    private allowExternalStore: Writable<boolean>;
    public readonly allowExternal: Readable<boolean>;
    private websocketPortStore: Writable<string>;
    public readonly websocketPort: Readable<string>;
    private authKeyStore: Writable<string>;
    public readonly authKey: Readable<string>;

    private store: Store | null = null;

    constructor() {
        this.isReadyStore = writable(false);
        this.isReady = { subscribe: this.isReadyStore.subscribe };
        this.isMobileStore = writable(false);
        this.isMobile = { subscribe: this.isMobileStore.subscribe };
        this.allowExternalStore = writable(false);
        this.allowExternal = { subscribe: this.allowExternalStore.subscribe };
        this.websocketPortStore = writable("9595");
        this.websocketPort = { subscribe: this.websocketPortStore.subscribe };
        this.authKeyStore = writable("");
        this.authKey = { subscribe: this.authKeyStore.subscribe };
    }

    async initialize(): Promise<void> {
        this.store = await AppStore.load();

        const mobile = await this.platformDetector.checkMobile().catch(() => false);
        this.isMobileStore.set(mobile);

        const config = await this.store.get<WebSocketConfig>("websocket_server");
        this.allowExternalStore.set(WebSocketSettingsManager.migrateAllowExternal(config, mobile));
        this.websocketPortStore.set(config?.port?.toString() || "9595");

        // The server is always running now, so nothing else will ever mint a key. First boot is
        // the only remaining moment that can.
        let key = config?.key ?? "";
        if (!key.trim()) {
            key = await invoke<string>("generate_encryption_key");
        }
        this.authKeyStore.set(key);

        // Written back unconditionally, so the migrated shape is on disk rather than recomputed
        // on every launch from fields that are meant to stop being read.
        await this.saveConfig();

        this.isReadyStore.set(true);
    }

    /**
     * Whether this installation ever asked to be reachable from another device.
     *
     * A user who never enabled the server never expressed a posture, so the absence of a choice
     * reads as no. On mobile the stored value was forced rather than chosen — the server bound
     * every interface because a phone had nothing local to serve — so it cannot be read as a
     * preference at all, and reading it as one would open every existing phone to the network.
     */
    private static migrateAllowExternal(
        config: WebSocketConfig | null | undefined,
        mobile: boolean,
    ): boolean {
        if (!config) return false;
        if (mobile) return false;
        if (typeof config.allow_external === "boolean") return config.allow_external;
        return Boolean(config.enabled) && !config.localhost_only;
    }

    async handleAllowExternalToggle(): Promise<void> {
        this.allowExternalStore.update((value) => !value);
        await this.saveConfig();
        Analytics.track("WebsocketExternalAccessToggled", {
            enabled: this.currentAllowExternal() ? 1 : 0,
        });
    }

    async handlePortChange(value: string): Promise<void> {
        this.websocketPortStore.set(value);
        await this.saveConfig();
    }

    async handleKeyChange(value: string): Promise<void> {
        this.authKeyStore.set(value);
        await this.saveConfig();
    }

    async handleGenerateKey(): Promise<void> {
        try {
            this.authKeyStore.set(await invoke<string>("generate_encryption_key"));
            await this.saveConfig();
        } catch (e) {
            error(`Failed to generate encryption key: ${e}`);
        }
    }

    /**
     * Persist the config, tell the backend, and rebind on it.
     *
     * The rebind is part of saving rather than a separate step a caller can forget: every field
     * here — the port, the token, the reach — is read at bind time and nowhere else, so a saved
     * change that did not rebind would show a setting the running listener does not have.
     */
    private async saveConfig(): Promise<void> {
        const allowExternal = this.currentAllowExternal();
        const config: WebSocketConfig = {
            enabled: true,
            localhost_only: !allowExternal,
            allow_external: allowExternal,
            port: parseInt(this.currentPort()),
            key: this.currentKey(),
        };
        await this.store?.set("websocket_server", config);
        await this.store?.save();
        await invoke("update_websocket_config", { config });

        try {
            await invoke("restart_websocket_external");
        } catch (e) {
            error(`Failed to rebind the WebSocket server: ${e}`);
        }
    }

    private currentAllowExternal(): boolean {
        let value = false;
        this.allowExternalStore.update((v) => {
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
}
