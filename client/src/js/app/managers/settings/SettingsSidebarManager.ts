import { writable, type Readable, type Writable } from "svelte/store";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import PlatformDetector from "../../utils/PlatformDetector";
import { BedrockManager } from "../bedrock/BedrockManager";
import type { RealmsGateStatus } from "../../../bindings/RealmsGateStatus";
import type { ApiConfigCheckResponse } from "../../../bindings/ApiConfigCheckResponse";

export class SettingsSidebarManager {
    private readonly platformDetector: PlatformDetector;

    private isMobileStore: Writable<boolean>;
    public readonly isMobile: Readable<boolean>;
    private currentPageTitleStore: Writable<string>;
    public readonly currentPageTitle: Readable<string>;
    private activePageStore: Writable<string>;
    public readonly activePage: Readable<string>;
    private realmsConnectEnabledStore: Writable<boolean>;
    public readonly realmsConnectEnabled: Readable<boolean>;
    private serverBedrockEnabledStore: Writable<boolean>;
    public readonly serverBedrockEnabled: Readable<boolean>;

    private readonly bedrockPageIds = new Set(["proxy_connect.svelte", "realms_connect.svelte"]);

    private bedrockManager: BedrockManager | null = null;
    private flagsUnlisten: UnlistenFn | null = null;
    private realmsConnectEnabledValue = false;

    constructor() {
        this.platformDetector = new PlatformDetector();

        this.isMobileStore = writable(false);
        this.isMobile = { subscribe: this.isMobileStore.subscribe };
        this.currentPageTitleStore = writable("Account");
        this.currentPageTitle = { subscribe: this.currentPageTitleStore.subscribe };
        this.activePageStore = writable("account.svelte");
        this.activePage = { subscribe: this.activePageStore.subscribe };
        this.realmsConnectEnabledStore = writable(false);
        this.realmsConnectEnabled = { subscribe: this.realmsConnectEnabledStore.subscribe };
        this.serverBedrockEnabledStore = writable(false);
        this.serverBedrockEnabled = { subscribe: this.serverBedrockEnabledStore.subscribe };
    }

    getBedrockManager(): BedrockManager {
        if (!this.bedrockManager) {
            this.bedrockManager = new BedrockManager();
        }
        return this.bedrockManager;
    }

    isBedrockPage(pageId: string): boolean {
        return this.bedrockPageIds.has(pageId);
    }

    setActivePage(pageId: string, title: string): void {
        this.activePageStore.set(pageId);
        this.currentPageTitleStore.set(title);
    }

    setCurrentPageTitle(title: string): void {
        this.currentPageTitleStore.set(title);
    }

    // Realms Connect is blocked until its feature flag is confirmed on. The page
    // also renders the subscription upsell internally when the user isn't entitled.
    canNavigateTo(pageId: string): boolean {
        if (pageId === "realms_connect.svelte" && !this.realmsConnectEnabledValue) {
            return false;
        }
        return true;
    }

    async initialize(initialActivePage: string, initialTitle: string): Promise<void> {
        this.activePageStore.set(initialActivePage);
        this.currentPageTitleStore.set(initialTitle);

        try {
            this.isMobileStore.set(await this.platformDetector.checkMobile());
        } catch {
            this.isMobileStore.set(false);
        }

        await this.refreshRealmsGate();
        this.flagsUnlisten = await listen("feature-flags-updated", () => {
            void this.refreshRealmsGate();
        });

        try {
            const check = await invoke<ApiConfigCheckResponse>("api_get_config");
            this.serverBedrockEnabledStore.set(check.config.bedrock.enabled);
        } catch {
            this.serverBedrockEnabledStore.set(false);
        }
    }

    // Re-evaluates the Realms Connect master flag. Run on mount and whenever the
    // backend signals feature flags changed (e.g. after a Discord re-sync), so
    // flag-gated UI appears without a restart.
    private async refreshRealmsGate(): Promise<void> {
        try {
            const gate = await invoke<RealmsGateStatus>("bedrock_realms_gate");
            this.realmsConnectEnabledValue = gate.status !== "feature_disabled";
        } catch {
            this.realmsConnectEnabledValue = false;
        }
        this.realmsConnectEnabledStore.set(this.realmsConnectEnabledValue);
    }

    destroy(): void {
        this.bedrockManager?.destroy();
        if (this.flagsUnlisten) {
            this.flagsUnlisten();
            this.flagsUnlisten = null;
        }
    }
}
