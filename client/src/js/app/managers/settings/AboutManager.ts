import { I18n } from "$lib/i18n";
import { writable, type Readable, type Writable } from "svelte/store";
import { invoke } from "@tauri-apps/api/core";
import { openUrl } from "@tauri-apps/plugin-opener";
import { error } from "@charlesportwoodii/tauri-plugin-curia";
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
    private isDevStore: Writable<boolean>;
    public readonly isDev: Readable<boolean>;

    private isExportingStore: Writable<boolean>;
    public readonly isExporting: Readable<boolean>;
    private exportErrorStore: Writable<string>;
    public readonly exportError: Readable<string>;

    private telemetryStore: Writable<boolean>;
    public readonly telemetry: Readable<boolean>;

    private platformIdStore: Writable<string>;
    public readonly platformId: Readable<string>;
    private isRefreshingPlatformIdStore: Writable<boolean>;
    public readonly isRefreshingPlatformId: Readable<boolean>;
    private platformIdErrorStore: Writable<string>;
    public readonly platformIdError: Readable<string>;

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
            title: I18n.t("Report a Bug"),
            description: I18n.t("Open a bug report on GitHub"),
            icon: `<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-2.5L13.732 4c-.77-.833-1.964-.833-2.732 0L4.082 16.5c-.77.833.192 2.5 1.732 2.5z"/>`,
        },
        {
            url: "https://discord.gg/MAHckcEATj",
            title: "Discussions",
            description: I18n.t("Community discussions and help"),
            icon: `<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M17 8h2a2 2 0 012 2v6a2 2 0 01-2 2h-2v4l-4-4H9a1.994 1.994 0 01-1.414-.586m0 0L11 14h4a2 2 0 002-2V6a2 2 0 00-2-2H5a2 2 0 00-2 2v6a2 2 0 002 2h2v4l.586-.586z"/>`,
        },
        {
            url: "https://raw.githubusercontent.com/Alaydriem/bedrock-voice-chat/refs/heads/master/PRIVACY_STATEMENT.md",
            title: I18n.t("Privacy Notice"),
            description: I18n.t("View privacy statement"),
            icon: `<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12l2 2 4-4m5.618-4.016A11.955 11.955 0 0112 2.944a11.955 11.955 0 01-8.618 3.04A12.02 12.02 0 003 9c0 5.591 3.824 10.29 9 11.622 5.176-1.332 9-6.03 9-11.622 0-1.042-.133-2.052-.382-3.016z"/>`,
        },
    ];

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
        this.isDevStore = writable(false);
        this.isDev = { subscribe: this.isDevStore.subscribe };

        this.isExportingStore = writable(false);
        this.isExporting = { subscribe: this.isExportingStore.subscribe };
        this.exportErrorStore = writable("");
        this.exportError = { subscribe: this.exportErrorStore.subscribe };

        this.telemetryStore = writable(true);
        this.telemetry = { subscribe: this.telemetryStore.subscribe };

        this.platformIdStore = writable("");
        this.platformId = { subscribe: this.platformIdStore.subscribe };
        this.isRefreshingPlatformIdStore = writable(false);
        this.isRefreshingPlatformId = { subscribe: this.isRefreshingPlatformIdStore.subscribe };
        this.platformIdErrorStore = writable("");
        this.platformIdError = { subscribe: this.platformIdErrorStore.subscribe };

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
            this.platformIdStore.set(await invoke<string>("get_platform_id"));
        } catch (e) {
            error(`Failed to get app info: ${e}`);
        }

        // Gates developer-only controls. A failure here means "not dev" rather
        // than an unhandled rejection: the pane must render either way.
        try {
            this.isDevStore.set((await invoke<string>("get_variant")) === "dev");
        } catch {
            this.isDevStore.set(false);
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

    /**
     * Trades the current identity for a new one. Analytics and feature flags follow it
     * without a restart, so the returned value is what the rest of the session reports
     * under.
     */
    async refreshPlatformId(): Promise<void> {
        this.isRefreshingPlatformIdStore.set(true);
        this.platformIdErrorStore.set("");
        try {
            this.platformIdStore.set(await invoke<string>("refresh_platform_id"));
        } catch (e) {
            this.platformIdErrorStore.set(String(e));
            error(`Failed to refresh the platform ID: ${e}`);
        } finally {
            this.isRefreshingPlatformIdStore.set(false);
        }
    }

    async handleRefreshFlags(): Promise<void> {
        this.isRefreshingFlagsStore.set(true);
        this.refreshFlagsMessageStore.set("");
        try {
            await this.featureFlags.refresh();
            this.refreshFlagsMessageStore.set(I18n.t("Feature flags refreshed."));
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
        window.removeEventListener("discord-link-updated", this.boundLoadDiscord);
    }
}
