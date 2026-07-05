import { writable, type Readable, type Writable } from "svelte/store";
import { invoke } from "@tauri-apps/api/core";
import { openUrl } from "@tauri-apps/plugin-opener";
import { error } from "@tauri-apps/plugin-log";
import { Store } from "@tauri-apps/plugin-store";
import Analytics from "../../analytics";
import PlatformDetector from "../../utils/PlatformDetector";
import FeatureFlagService from "../../services/FeatureFlagService";
import type { AppInfo } from "../../../bindings/AppInfo";
import type { DiscordLinkStatus } from "../../../bindings/DiscordLinkStatus";
import type { AboutLink } from "./AboutLink";

type DiscordCommand = "discord_link" | "discord_resync" | "discord_unlink";

export class AboutManager {
    private readonly featureFlags: FeatureFlagService;
    private readonly platformDetector: PlatformDetector;

    private appInfoStore: Writable<AppInfo | null>;
    public readonly appInfo: Readable<AppInfo | null>;
    private isReadyStore: Writable<boolean>;
    public readonly isReady: Readable<boolean>;
    private isMobileStore: Writable<boolean>;
    public readonly isMobile: Readable<boolean>;

    private isExportingStore: Writable<boolean>;
    public readonly isExporting: Readable<boolean>;
    private exportErrorStore: Writable<string>;
    public readonly exportError: Readable<string>;

    private telemetryStore: Writable<boolean>;
    public readonly telemetry: Readable<boolean>;

    private showPlatformIdStore: Writable<boolean>;
    public readonly showPlatformId: Readable<boolean>;
    private platformIdStore: Writable<string>;
    public readonly platformId: Readable<string>;
    private platformIdCopiedStore: Writable<boolean>;
    public readonly platformIdCopied: Readable<boolean>;

    private isRefreshingFlagsStore: Writable<boolean>;
    public readonly isRefreshingFlags: Readable<boolean>;
    private refreshFlagsMessageStore: Writable<string>;
    public readonly refreshFlagsMessage: Readable<string>;

    private discordStore: Writable<DiscordLinkStatus | null>;
    public readonly discord: Readable<DiscordLinkStatus | null>;
    private discordBusyStore: Writable<boolean>;
    public readonly discordBusy: Readable<boolean>;
    private discordErrorStore: Writable<string>;
    public readonly discordError: Readable<string>;

    public readonly links: AboutLink[] = [
        {
            url: "https://github.com/alaydriem/bedrock-voice-chat/issues",
            title: "Report a Bug",
            description: "Open a bug report on GitHub",
            icon: `<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-2.5L13.732 4c-.77-.833-1.964-.833-2.732 0L4.082 16.5c-.77.833.192 2.5 1.732 2.5z"/>`,
        },
        {
            url: "https://discord.gg/MAHckcEATj",
            title: "Discussions",
            description: "Community discussions and help",
            icon: `<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M17 8h2a2 2 0 012 2v6a2 2 0 01-2 2h-2v4l-4-4H9a1.994 1.994 0 01-1.414-.586m0 0L11 14h4a2 2 0 002-2V6a2 2 0 00-2-2H5a2 2 0 00-2 2v6a2 2 0 002 2h2v4l.586-.586z"/>`,
        },
        {
            url: "https://raw.githubusercontent.com/Alaydriem/bedrock-voice-chat/refs/heads/master/PRIVACY_STATEMENT.md",
            title: "Privacy Notice",
            description: "View privacy statement",
            icon: `<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12l2 2 4-4m5.618-4.016A11.955 11.955 0 0112 2.944a11.955 11.955 0 01-8.618 3.04A12.02 12.02 0 003 9c0 5.591 3.824 10.29 9 11.622 5.176-1.332 9-6.03 9-11.622 0-1.042-.133-2.052-.382-3.016z"/>`,
        },
    ];

    private variantClickCount = 0;
    private variantClickTimer: ReturnType<typeof setTimeout> | null = null;
    private platformIdCopiedTimer: ReturnType<typeof setTimeout> | null = null;
    private readonly boundLoadDiscord: () => void;

    constructor() {
        this.featureFlags = new FeatureFlagService();
        this.platformDetector = new PlatformDetector();

        this.appInfoStore = writable(null);
        this.appInfo = { subscribe: this.appInfoStore.subscribe };
        this.isReadyStore = writable(false);
        this.isReady = { subscribe: this.isReadyStore.subscribe };
        this.isMobileStore = writable(false);
        this.isMobile = { subscribe: this.isMobileStore.subscribe };

        this.isExportingStore = writable(false);
        this.isExporting = { subscribe: this.isExportingStore.subscribe };
        this.exportErrorStore = writable("");
        this.exportError = { subscribe: this.exportErrorStore.subscribe };

        this.telemetryStore = writable(true);
        this.telemetry = { subscribe: this.telemetryStore.subscribe };

        this.showPlatformIdStore = writable(false);
        this.showPlatformId = { subscribe: this.showPlatformIdStore.subscribe };
        this.platformIdStore = writable("");
        this.platformId = { subscribe: this.platformIdStore.subscribe };
        this.platformIdCopiedStore = writable(false);
        this.platformIdCopied = { subscribe: this.platformIdCopiedStore.subscribe };

        this.isRefreshingFlagsStore = writable(false);
        this.isRefreshingFlags = { subscribe: this.isRefreshingFlagsStore.subscribe };
        this.refreshFlagsMessageStore = writable("");
        this.refreshFlagsMessage = { subscribe: this.refreshFlagsMessageStore.subscribe };

        this.discordStore = writable(null);
        this.discord = { subscribe: this.discordStore.subscribe };
        this.discordBusyStore = writable(false);
        this.discordBusy = { subscribe: this.discordBusyStore.subscribe };
        this.discordErrorStore = writable("");
        this.discordError = { subscribe: this.discordErrorStore.subscribe };

        this.boundLoadDiscord = () => {
            void this.loadDiscord();
        };
    }

    async initialize(): Promise<void> {
        this.isMobileStore.set(await this.platformDetector.checkMobile());

        try {
            this.appInfoStore.set(await invoke<AppInfo>("get_app_info"));
            this.telemetryStore.set(await invoke<boolean>("get_telemetry"));
        } catch (e) {
            error(`Failed to get app info: ${e}`);
        }

        await this.loadDiscord();
        window.addEventListener("discord-link-updated", this.boundLoadDiscord);
        this.isReadyStore.set(true);
    }

    openLink(url: string): void {
        void openUrl(url);
    }

    private async loadDiscord(): Promise<void> {
        try {
            this.discordStore.set(await invoke<DiscordLinkStatus>("discord_status"));
        } catch (e) {
            error(`Failed to get Discord status: ${e}`);
        }
    }

    async discordAction(cmd: DiscordCommand): Promise<void> {
        this.discordBusyStore.set(true);
        this.discordErrorStore.set("");
        try {
            this.discordStore.set(await invoke<DiscordLinkStatus>(cmd));
        } catch (e) {
            this.discordErrorStore.set(String(e));
            error(`${cmd} failed: ${e}`);
        } finally {
            this.discordBusyStore.set(false);
        }
    }

    async handleVariantClick(): Promise<void> {
        this.variantClickCount++;

        if (this.variantClickTimer) clearTimeout(this.variantClickTimer);
        this.variantClickTimer = setTimeout(() => {
            this.variantClickCount = 0;
        }, 2000);

        let revealed = false;
        this.showPlatformIdStore.update((value) => {
            revealed = value;
            return value;
        });

        if (this.variantClickCount >= 3 && !revealed) {
            try {
                const store = await Store.load("store.json", { autoSave: false, defaults: {} });
                this.platformIdStore.set((await store.get<string>("install_id")) ?? "");
            } catch (e) {
                error(`Failed to read install_id: ${e}`);
            }
            this.showPlatformIdStore.set(true);
        }
    }

    async copyPlatformId(): Promise<void> {
        let id = "";
        this.platformIdStore.update((value) => {
            id = value;
            return value;
        });
        try {
            await navigator.clipboard.writeText(id);
            this.platformIdCopiedStore.set(true);
            if (this.platformIdCopiedTimer) clearTimeout(this.platformIdCopiedTimer);
            this.platformIdCopiedTimer = setTimeout(() => {
                this.platformIdCopiedStore.set(false);
            }, 1500);
        } catch (e) {
            error(`Failed to copy install_id: ${e}`);
        }
    }

    async handleRefreshFlags(): Promise<void> {
        this.isRefreshingFlagsStore.set(true);
        this.refreshFlagsMessageStore.set("");
        try {
            await this.featureFlags.refresh();
            this.refreshFlagsMessageStore.set("Feature flags refreshed.");
        } catch (e) {
            this.refreshFlagsMessageStore.set(`Refresh failed: ${e}`);
            error(`Refresh feature flags failed: ${e}`);
        } finally {
            this.isRefreshingFlagsStore.set(false);
        }
    }

    async handleExportLogs(): Promise<void> {
        this.isExportingStore.set(true);
        this.exportErrorStore.set("");
        try {
            await invoke<boolean>("export_logs");
        } catch (e) {
            this.exportErrorStore.set(String(e));
            error(`Failed to export logs ${e}`);
        } finally {
            this.isExportingStore.set(false);
        }
    }

    async handleTelemetryToggle(): Promise<void> {
        let current = true;
        this.telemetryStore.update((value) => {
            current = value;
            return value;
        });

        if (current) {
            Analytics.track("AnalyticsToggled", { enabled: 0 });
        }
        const next = !current;
        this.telemetryStore.set(next);
        await invoke("set_telemetry", { value: next });
        if (next) {
            Analytics.track("AnalyticsToggled", { enabled: 1 });
        }
    }

    destroy(): void {
        if (this.variantClickTimer) clearTimeout(this.variantClickTimer);
        if (this.platformIdCopiedTimer) clearTimeout(this.platformIdCopiedTimer);
        window.removeEventListener("discord-link-updated", this.boundLoadDiscord);
    }
}
