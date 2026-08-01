import { writable, type Readable, type Writable } from "svelte/store";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import PlatformDetector from "../../utils/PlatformDetector";
import { BedrockManager } from "../bedrock/BedrockManager";
import { BedrockCapabilityManager } from "../bedrock/BedrockCapabilityManager";

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

    // Whether the connected BVC server supports Bedrock features, plus the
    // operator-curated proxy server list. Shared with the Bedrock pages via
    // BedrockManager.
    public readonly capability: BedrockCapabilityManager;

    private readonly bedrockPageIds = new Set(["proxy_connect.svelte", "realms_connect.svelte"]);

    private bedrockManager: BedrockManager | null = null;
    private flagsUnlisten: UnlistenFn | null = null;
    private realmsConnectEnabledValue = true;

    constructor() {
        this.platformDetector = new PlatformDetector();

        this.isMobileStore = writable(false);
        this.isMobile = { subscribe: this.isMobileStore.subscribe };
        this.currentPageTitleStore = writable("Account");
        this.currentPageTitle = { subscribe: this.currentPageTitleStore.subscribe };
        this.activePageStore = writable("account.svelte");
        this.activePage = { subscribe: this.activePageStore.subscribe };
        this.realmsConnectEnabledStore = writable(true);
        this.realmsConnectEnabled = { subscribe: this.realmsConnectEnabledStore.subscribe };
        this.capability = new BedrockCapabilityManager();
    }

    getBedrockManager(): BedrockManager {
        if (!this.bedrockManager) {
            this.bedrockManager = new BedrockManager(this.capability);
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

    // Realms Connect is hidden while its kill switch is off.
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

        await this.refreshRealmsEnabled();
        this.flagsUnlisten = await listen("feature-flags-updated", () => {
            void this.refreshRealmsEnabled();
        });

        await this.capability.refresh();
    }

    // Re-reads the Realms Connect kill switch. Run on mount and whenever the
    // backend signals feature flags changed (e.g. after a Discord re-sync), so
    // the sidebar item appears without a restart. Fails open: a failed read
    // must not hide a free feature.
    private async refreshRealmsEnabled(): Promise<void> {
        try {
            this.realmsConnectEnabledValue = await invoke<boolean>("bedrock_realms_enabled");
        } catch {
            this.realmsConnectEnabledValue = true;
        }
        this.realmsConnectEnabledStore.set(this.realmsConnectEnabledValue);
    }

    destroy(): void {
        this.bedrockManager?.destroy();
        this.capability.destroy();
        if (this.flagsUnlisten) {
            this.flagsUnlisten();
            this.flagsUnlisten = null;
        }
    }
}
