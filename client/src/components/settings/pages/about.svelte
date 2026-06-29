<script lang="ts">
    import { onMount, onDestroy } from "svelte";
    import { invoke } from "@tauri-apps/api/core";
    import { openUrl } from "@tauri-apps/plugin-opener";
    import { error } from "@tauri-apps/plugin-log";
    import { Store } from "@tauri-apps/plugin-store";
    import Analytics from "../../../js/app/analytics";
    import PlatformDetector from "../../../js/app/utils/PlatformDetector.ts";
    import FeatureFlagService from "../../../js/app/services/FeatureFlagService.ts";
    import type { DiscordLinkStatus } from "../../../js/bindings/DiscordLinkStatus";

    interface AppInfo {
        app_version: string;
        protocol_version: string;
        build_commit: string;
        build_variant: string;
        build_number: string;
    }

    interface AboutLink {
        url: string;
        title: string;
        description: string;
        icon: string;
    }

    const featureFlagService = new FeatureFlagService();

    const links: AboutLink[] = [
        {
            url: "https://github.com/alaydriem/bedrock-voice-chat/issues",
            title: "Report a Bug",
            description: "Open a bug report on GitHub",
            icon: `<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-2.5L13.732 4c-.77-.833-1.964-.833-2.732 0L4.082 16.5c-.77.833.192 2.5 1.732 2.5z"/>`
        },
        {
            url: "https://discord.gg/MAHckcEATj",
            title: "Discussions",
            description: "Community discussions and help",
            icon: `<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M17 8h2a2 2 0 012 2v6a2 2 0 01-2 2h-2v4l-4-4H9a1.994 1.994 0 01-1.414-.586m0 0L11 14h4a2 2 0 002-2V6a2 2 0 00-2-2H5a2 2 0 00-2 2v6a2 2 0 002 2h2v4l.586-.586z"/>`
        },
        {
            url: "https://raw.githubusercontent.com/Alaydriem/bedrock-voice-chat/refs/heads/master/PRIVACY_STATEMENT.md",
            title: "Privacy Notice",
            description: "View privacy statement",
            icon: `<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12l2 2 4-4m5.618-4.016A11.955 11.955 0 0112 2.944a11.955 11.955 0 01-8.618 3.04A12.02 12.02 0 003 9c0 5.591 3.824 10.29 9 11.622 5.176-1.332 9-6.03 9-11.622 0-1.042-.133-2.052-.382-3.016z"/>`
        }
    ];

    let appInfo: AppInfo | null = $state(null);
    let isReady = $state(false);
    let isMobile = $state(false);
    let isExporting = $state(false);
    let exportError = $state("");
    let telemetry = $state(true);

    let variantClickCount = $state(0);
    let variantClickTimer: ReturnType<typeof setTimeout> | null = null;

    // Page-local only (deliberately not persisted): tap the build variant 3x to
    // reveal the install_id, used to target this install in Flagsmith.
    let showPlatformId = $state(false);
    let platformId = $state("");
    let platformIdCopied = $state(false);

    let isRefreshingFlags = $state(false);
    let refreshFlagsMessage = $state("");

    let discord: DiscordLinkStatus | null = $state(null);
    let discordBusy = $state(false);
    let discordError = $state("");

    async function loadDiscord() {
        try {
            discord = await invoke<DiscordLinkStatus>("discord_status");
        } catch (e) {
            error(`Failed to get Discord status: ${e}`);
        }
    }

    async function discordAction(cmd: "discord_link" | "discord_resync" | "discord_unlink") {
        discordBusy = true;
        discordError = "";
        try {
            discord = await invoke<DiscordLinkStatus>(cmd);
        } catch (e) {
            discordError = String(e);
            error(`${cmd} failed: ${e}`);
        } finally {
            discordBusy = false;
        }
    }

    async function handleVariantClick() {
        variantClickCount++;

        if (variantClickTimer) clearTimeout(variantClickTimer);
        variantClickTimer = setTimeout(() => { variantClickCount = 0; }, 2000);

        if (variantClickCount >= 3 && !showPlatformId) {
            try {
                const store = await Store.load("store.json", { autoSave: false });
                platformId = (await store.get<string>("install_id")) ?? "";
            } catch (e) {
                error(`Failed to read install_id: ${e}`);
            }
            showPlatformId = true;
        }
    }

    async function copyPlatformId() {
        try {
            await navigator.clipboard.writeText(platformId);
            platformIdCopied = true;
            setTimeout(() => { platformIdCopied = false; }, 1500);
        } catch (e) {
            error(`Failed to copy install_id: ${e}`);
        }
    }

    async function handleRefreshFlags() {
        isRefreshingFlags = true;
        refreshFlagsMessage = "";
        try {
            await featureFlagService.refresh();
            refreshFlagsMessage = "Feature flags refreshed.";
        } catch (e) {
            refreshFlagsMessage = `Refresh failed: ${e}`;
            error(`Refresh feature flags failed: ${e}`);
        } finally {
            isRefreshingFlags = false;
        }
    }

    onDestroy(() => {
        if (variantClickTimer) clearTimeout(variantClickTimer);
        window.removeEventListener("discord-link-updated", loadDiscord);
    });

    async function handleExportLogs() {
        isExporting = true;
        exportError = "";
        try {
            await invoke<boolean>("export_logs");
        } catch (e) {
            exportError = String(e);
            error(`Failed to export logs ${e}`);
        } finally {
            isExporting = false;
        }
    }

    async function handleTelemetryToggle() {
        if (telemetry) {
            Analytics.track("AnalyticsToggled", { enabled: 0 });
        }
        telemetry = !telemetry;
        await invoke("set_telemetry", { value: telemetry });
        if (telemetry) {
            Analytics.track("AnalyticsToggled", { enabled: 1 });
        }
    }

    onMount(async () => {
        const platformDetector = new PlatformDetector();
        isMobile = await platformDetector.checkMobile();

        try {
            appInfo = await invoke<AppInfo>("get_app_info");
            telemetry = await invoke<boolean>("get_telemetry");
        } catch (e) {
            error(`Failed to get app info: ${e}`);
        }
        await loadDiscord();
        window.addEventListener("discord-link-updated", loadDiscord);
        isReady = true;
    });
</script>

<div class="grid grid-cols-1 gap-4 sm:gap-5 lg:gap-6 pt-4 md:pt-0">
    <!-- App Information -->
    <div class="card px-5 pb-4 sm:px-5">
        <div class="my-3 flex flex-col">
            <h2 class="font-medium tracking-wide text-slate-700 dark:text-navy-100 lg:text-base pb-2">
                App Information
            </h2>
            <p class="text-sm leading-6 hidden md:block">
                Version and build details for Bedrock Voice Chat
            </p>
        </div>

        {#if isReady && appInfo}
        <div class="space-y-1 mt-2">
            <div class="flex items-center justify-between py-2 px-3 rounded-lg hover:bg-slate-50 dark:hover:bg-navy-600">
                <span class="text-sm font-medium text-slate-700 dark:text-navy-100">App Version</span>
                <span class="text-sm text-slate-500 dark:text-navy-300 font-mono">v{appInfo.app_version}</span>
            </div>
            <div class="flex items-center justify-between py-2 px-3 rounded-lg hover:bg-slate-50 dark:hover:bg-navy-600">
                <span class="text-sm font-medium text-slate-700 dark:text-navy-100">Protocol Version</span>
                <span class="text-sm text-slate-500 dark:text-navy-300 font-mono">{appInfo.protocol_version}</span>
            </div>
            <div class="flex items-center justify-between py-2 px-3 rounded-lg hover:bg-slate-50 dark:hover:bg-navy-600">
                <span class="text-sm font-medium text-slate-700 dark:text-navy-100">Build Commit</span>
                <span class="text-sm text-slate-500 dark:text-navy-300 font-mono">{appInfo.build_commit}</span>
            </div>
            <div class="flex items-center justify-between py-2 px-3 rounded-lg hover:bg-slate-50 dark:hover:bg-navy-600">
                <span class="text-sm font-medium text-slate-700 dark:text-navy-100">Build Number</span>
                <span class="text-sm text-slate-500 dark:text-navy-300 font-mono">{appInfo.build_number}</span>
            </div>
            <div class="flex items-center justify-between py-2 px-3 rounded-lg hover:bg-slate-50 dark:hover:bg-navy-600">
                <span class="text-sm font-medium text-slate-700 dark:text-navy-100">Build Variant</span>
                <span
                    class="badge {appInfo.build_variant === 'dev' ? 'bg-warning text-white' : 'bg-success text-white'} cursor-pointer select-none"
                    onclick={handleVariantClick}
                    role="button"
                    tabindex="0"
                >
                    {appInfo.build_variant}
                </span>
            </div>
            {#if showPlatformId}
            <div class="flex items-center justify-between py-2 px-3 rounded-lg hover:bg-slate-50 dark:hover:bg-navy-600">
                <span class="text-sm font-medium text-slate-700 dark:text-navy-100">Platform ID</span>
                <button
                    class="text-sm text-slate-500 dark:text-navy-300 font-mono cursor-pointer hover:text-primary dark:hover:text-accent-light"
                    onclick={copyPlatformId}
                    title="Click to copy"
                >
                    {platformIdCopied ? "Copied!" : platformId}
                </button>
            </div>
            {/if}
        </div>
        {/if}
    </div>

    <!-- Links -->
    <div class="card px-5 pb-4 sm:px-5">
        <div class="my-3 flex flex-col">
            <h2 class="font-medium tracking-wide text-slate-700 dark:text-navy-100 lg:text-base pb-2">
                Links
            </h2>
        </div>

        <div class="space-y-1 mt-2">
            {#each links as link}
            <button
                class="flex w-full items-center justify-between py-3 px-4 rounded-lg hover:bg-slate-50 dark:hover:bg-navy-600 transition-colors text-left"
                onclick={() => openUrl(link.url)}
            >
                <div class="flex items-center space-x-3">
                    <svg xmlns="http://www.w3.org/2000/svg" class="size-5 text-slate-400 dark:text-navy-300" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                        {@html link.icon}
                    </svg>
                    <div>
                        <span class="text-sm font-medium text-slate-700 dark:text-navy-100">{link.title}</span>
                        <p class="text-xs text-slate-500 dark:text-navy-300 mt-0.5">{link.description}</p>
                    </div>
                </div>
                <svg xmlns="http://www.w3.org/2000/svg" class="size-4 text-slate-400" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10 6H6a2 2 0 00-2 2v10a2 2 0 002 2h10a2 2 0 002-2v-4M14 4h6m0 0v6m0-6L10 14"/>
                </svg>
            </button>
            {/each}
        </div>
    </div>

    <!-- Privacy & Telemetry -->
    <div class="card px-5 pb-4 sm:px-5">
        <div class="my-3 flex flex-col">
            <h2 class="font-medium tracking-wide text-slate-700 dark:text-navy-100 lg:text-base pb-2">
                Privacy & Telemetry
            </h2>
            <p class="text-sm leading-6 hidden md:block">
                Control whether BVC sends crash reports and usage data.
            </p>
        </div>

        <div class="space-y-1 mt-2">
            <div class="flex items-center justify-between py-2 px-3 rounded-lg hover:bg-slate-50 dark:hover:bg-navy-600">
                <div>
                    <span class="text-sm font-medium text-slate-700 dark:text-navy-100">Error Reporting & Analytics</span>
                    <p class="text-xs text-slate-500 dark:text-navy-300 mt-0.5">
                        Send crash reports and usage data to help improve BVC.
                        {#if !telemetry}
                            <span class="text-warning font-medium">Telemetry is currently disabled.</span>
                        {/if}
                    </p>
                </div>
                <input
                    type="checkbox"
                    checked={telemetry}
                    onchange={handleTelemetryToggle}
                    class="form-switch h-5 w-10 rounded-full bg-slate-300 before:rounded-full before:bg-slate-50 checked:bg-primary checked:before:bg-white dark:bg-navy-900 dark:before:bg-navy-300 dark:checked:bg-accent dark:checked:before:bg-white"
                />
            </div>
        </div>
    </div>

    <!-- Discord Role Features -->
    {#if discord?.configured}
    <div class="card px-5 pb-4 sm:px-5">
        <div class="my-3 flex flex-col">
            <h2 class="font-medium tracking-wide text-slate-700 dark:text-navy-100 lg:text-base pb-2">
                Enable Additional Features via Linked Discord Role
            </h2>
            <p class="text-sm leading-6">
                Link your Discord account to unlock features tied to your server roles.
            </p>
        </div>

        <div class="mt-2 space-y-2">
            {#if discord.linked && !discord.expired}
                <p class="text-sm text-slate-500 dark:text-navy-300">
                    Linked · {discord.role_count} role{discord.role_count === 1 ? "" : "s"}
                </p>
            {:else if discord.linked && discord.expired}
                <p class="text-sm text-warning">Discord roles expired — re-sync to restore features.</p>
            {:else}
                <p class="text-sm text-slate-500 dark:text-navy-300">Not linked.</p>
            {/if}

            <div class="flex gap-2">
                {#if !discord.linked}
                    <button
                        class="btn bg-primary font-medium text-white hover:bg-primary-focus dark:bg-accent dark:hover:bg-accent-focus"
                        onclick={() => discordAction("discord_link")}
                        disabled={discordBusy}
                    >
                        {discordBusy ? "Linking…" : "Link Discord"}
                    </button>
                {:else}
                    <button
                        class="btn bg-primary font-medium text-white hover:bg-primary-focus dark:bg-accent dark:hover:bg-accent-focus"
                        onclick={() => discordAction("discord_resync")}
                        disabled={discordBusy}
                    >
                        {discordBusy ? "Re-syncing…" : "Re-sync"}
                    </button>
                    <button
                        class="btn border border-slate-300 font-medium dark:border-navy-450"
                        onclick={() => discordAction("discord_unlink")}
                        disabled={discordBusy}
                    >
                        Unlink
                    </button>
                {/if}
            </div>
            {#if discordError}
                <p class="text-xs text-error">{discordError}</p>
            {/if}
        </div>
    </div>
    {/if}

    <!-- Diagnostics -->
    {#if !isMobile}
    <div class="card px-5 pb-4 sm:px-5">
        <div class="my-3 flex flex-col">
            <h2 class="font-medium tracking-wide text-slate-700 dark:text-navy-100 lg:text-base pb-2">
                Diagnostics
            </h2>
            <p class="text-sm leading-6">
                Export application logs to share with developers when reporting issues.
            </p>
        </div>

        <div class="mt-2">
            <button
                class="btn bg-primary font-medium text-white hover:bg-primary-focus dark:bg-accent dark:hover:bg-accent-focus"
                onclick={handleExportLogs}
                disabled={isExporting}
            >
                {#if isExporting}
                    <svg class="animate-spin -ml-1 mr-2 h-4 w-4 text-white" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24">
                        <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
                        <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4z"></path>
                    </svg>
                    Exporting...
                {:else}
                    Export Logs
                {/if}
            </button>
            {#if exportError}
                <p class="text-xs text-error mt-2">{exportError}</p>
            {/if}
        </div>
    </div>
    {/if}

    <!-- Developer (debug builds only) -->
    {#if appInfo?.build_variant === "dev"}
    <div class="card px-5 pb-4 sm:px-5">
        <div class="my-3 flex flex-col">
            <h2 class="font-medium tracking-wide text-slate-700 dark:text-navy-100 lg:text-base pb-2">
                Developer
            </h2>
            <p class="text-sm leading-6">
                Debug-only tools. Re-fetch feature flags from Flagsmith without restarting.
            </p>
        </div>

        <div class="mt-2">
            <button
                class="btn bg-primary font-medium text-white hover:bg-primary-focus dark:bg-accent dark:hover:bg-accent-focus"
                onclick={handleRefreshFlags}
                disabled={isRefreshingFlags}
            >
                {isRefreshingFlags ? "Refreshing…" : "Refresh Feature Flags"}
            </button>
            {#if refreshFlagsMessage}
                <p class="text-xs text-slate-500 dark:text-navy-300 mt-2">{refreshFlagsMessage}</p>
            {/if}
        </div>
    </div>
    {/if}
</div>
