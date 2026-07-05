<script lang="ts">
    import { onMount, onDestroy } from "svelte";
    import { AboutManager } from "../../../js/app/managers/settings/AboutManager";

    const manager = new AboutManager();

    const appInfo = manager.appInfo;
    const isReady = manager.isReady;
    const isMobile = manager.isMobile;
    const isExporting = manager.isExporting;
    const exportError = manager.exportError;
    const telemetry = manager.telemetry;
    const showPlatformId = manager.showPlatformId;
    const platformId = manager.platformId;
    const platformIdCopied = manager.platformIdCopied;
    const isRefreshingFlags = manager.isRefreshingFlags;
    const refreshFlagsMessage = manager.refreshFlagsMessage;
    const discord = manager.discord;
    const discordBusy = manager.discordBusy;
    const discordError = manager.discordError;

    onMount(() => {
        manager.initialize();
    });

    onDestroy(() => {
        manager.destroy();
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

        {#if $isReady && $appInfo}
        <div class="space-y-1 mt-2">
            <div class="flex items-center justify-between py-2 px-3 rounded-lg hover:bg-slate-50 dark:hover:bg-navy-600">
                <span class="text-sm font-medium text-slate-700 dark:text-navy-100">App Version</span>
                <span class="text-sm text-slate-500 dark:text-navy-300 font-mono">v{$appInfo.app_version}</span>
            </div>
            <div class="flex items-center justify-between py-2 px-3 rounded-lg hover:bg-slate-50 dark:hover:bg-navy-600">
                <span class="text-sm font-medium text-slate-700 dark:text-navy-100">Protocol Version</span>
                <span class="text-sm text-slate-500 dark:text-navy-300 font-mono">{$appInfo.protocol_version}</span>
            </div>
            <div class="flex items-center justify-between py-2 px-3 rounded-lg hover:bg-slate-50 dark:hover:bg-navy-600">
                <span class="text-sm font-medium text-slate-700 dark:text-navy-100">Build Commit</span>
                <span class="text-sm text-slate-500 dark:text-navy-300 font-mono">{$appInfo.build_commit}</span>
            </div>
            <div class="flex items-center justify-between py-2 px-3 rounded-lg hover:bg-slate-50 dark:hover:bg-navy-600">
<<<<<<< Updated upstream
                <span class="text-sm font-medium text-slate-700 dark:text-navy-100">Build Number</span>
                <span class="text-sm text-slate-500 dark:text-navy-300 font-mono">{appInfo.build_number}</span>
            </div>
            <div class="flex items-center justify-between py-2 px-3 rounded-lg hover:bg-slate-50 dark:hover:bg-navy-600">
=======
<<<<<<< Updated upstream
=======
                <span class="text-sm font-medium text-slate-700 dark:text-navy-100">Build Number</span>
                <span class="text-sm text-slate-500 dark:text-navy-300 font-mono">{$appInfo.build_number}</span>
            </div>
            <div class="flex items-center justify-between py-2 px-3 rounded-lg hover:bg-slate-50 dark:hover:bg-navy-600">
>>>>>>> Stashed changes
>>>>>>> Stashed changes
                <span class="text-sm font-medium text-slate-700 dark:text-navy-100">Build Variant</span>
                <span
                    class="badge {$appInfo.build_variant === 'dev' ? 'bg-warning text-white' : 'bg-success text-white'} cursor-pointer select-none"
                    onclick={() => manager.handleVariantClick()}
                    role="button"
                    tabindex="0"
                >
                    {$appInfo.build_variant}
                </span>
            </div>
            {#if $showPlatformId}
            <div class="flex items-center justify-between py-2 px-3 rounded-lg hover:bg-slate-50 dark:hover:bg-navy-600">
                <span class="text-sm font-medium text-slate-700 dark:text-navy-100">Platform ID</span>
                <button
                    class="text-sm text-slate-500 dark:text-navy-300 font-mono cursor-pointer hover:text-primary dark:hover:text-accent-light"
                    onclick={() => manager.copyPlatformId()}
                    title="Click to copy"
                >
                    {$platformIdCopied ? "Copied!" : $platformId}
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
            {#each manager.links as link}
            <button
                class="flex w-full items-center justify-between py-3 px-4 rounded-lg hover:bg-slate-50 dark:hover:bg-navy-600 transition-colors text-left"
                onclick={() => manager.openLink(link.url)}
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
                        {#if !$telemetry}
                            <span class="text-warning font-medium">Telemetry is currently disabled.</span>
                        {/if}
                    </p>
                </div>
                <input
                    type="checkbox"
                    checked={$telemetry}
                    onchange={() => manager.handleTelemetryToggle()}
                    class="form-switch h-5 w-10 rounded-full bg-slate-300 before:rounded-full before:bg-slate-50 checked:bg-primary checked:before:bg-white dark:bg-navy-900 dark:before:bg-navy-300 dark:checked:bg-accent dark:checked:before:bg-white"
                />
            </div>
        </div>
    </div>

<<<<<<< Updated upstream
    <!-- Discord Role Features -->
    {#if discord?.configured}
=======
<<<<<<< Updated upstream
=======
    <!-- Discord Role Features -->
    {#if $discord?.configured}
>>>>>>> Stashed changes
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
<<<<<<< Updated upstream
            {#if discord.linked && !discord.expired}
                <p class="text-sm text-slate-500 dark:text-navy-300">
                    Linked · {discord.role_count} role{discord.role_count === 1 ? "" : "s"}
                </p>
            {:else if discord.linked && discord.expired}
=======
            {#if $discord.linked && !$discord.expired}
                <p class="text-sm text-slate-500 dark:text-navy-300">
                    Linked · {$discord.role_count} role{$discord.role_count === 1 ? "" : "s"}
                </p>
            {:else if $discord.linked && $discord.expired}
>>>>>>> Stashed changes
                <p class="text-sm text-warning">Discord roles expired — re-sync to restore features.</p>
            {:else}
                <p class="text-sm text-slate-500 dark:text-navy-300">Not linked.</p>
            {/if}

            <div class="flex gap-2">
<<<<<<< Updated upstream
                {#if !discord.linked}
                    <button
                        class="btn bg-primary font-medium text-white hover:bg-primary-focus dark:bg-accent dark:hover:bg-accent-focus"
                        onclick={() => discordAction("discord_link")}
                        disabled={discordBusy}
                    >
                        {discordBusy ? "Linking…" : "Link Discord"}
=======
                {#if !$discord.linked}
                    <button
                        class="btn bg-primary font-medium text-white hover:bg-primary-focus dark:bg-accent dark:hover:bg-accent-focus"
                        onclick={() => manager.discordAction("discord_link")}
                        disabled={$discordBusy}
                    >
                        {$discordBusy ? "Linking…" : "Link Discord"}
>>>>>>> Stashed changes
                    </button>
                {:else}
                    <button
                        class="btn bg-primary font-medium text-white hover:bg-primary-focus dark:bg-accent dark:hover:bg-accent-focus"
<<<<<<< Updated upstream
                        onclick={() => discordAction("discord_resync")}
                        disabled={discordBusy}
                    >
                        {discordBusy ? "Re-syncing…" : "Re-sync"}
                    </button>
                    <button
                        class="btn border border-slate-300 font-medium dark:border-navy-450"
                        onclick={() => discordAction("discord_unlink")}
                        disabled={discordBusy}
=======
                        onclick={() => manager.discordAction("discord_resync")}
                        disabled={$discordBusy}
                    >
                        {$discordBusy ? "Re-syncing…" : "Re-sync"}
                    </button>
                    <button
                        class="btn border border-slate-300 font-medium dark:border-navy-450"
                        onclick={() => manager.discordAction("discord_unlink")}
                        disabled={$discordBusy}
>>>>>>> Stashed changes
                    >
                        Unlink
                    </button>
                {/if}
            </div>
<<<<<<< Updated upstream
            {#if discordError}
                <p class="text-xs text-error">{discordError}</p>
=======
            {#if $discordError}
                <p class="text-xs text-error">{$discordError}</p>
>>>>>>> Stashed changes
            {/if}
        </div>
    </div>
    {/if}

<<<<<<< Updated upstream
=======
>>>>>>> Stashed changes
>>>>>>> Stashed changes
    <!-- Diagnostics -->
    {#if !$isMobile}
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
                onclick={() => manager.handleExportLogs()}
                disabled={$isExporting}
            >
                {#if $isExporting}
                    <svg class="animate-spin -ml-1 mr-2 h-4 w-4 text-white" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24">
                        <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
                        <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4z"></path>
                    </svg>
                    Exporting...
                {:else}
                    Export Logs
                {/if}
            </button>
            {#if $exportError}
                <p class="text-xs text-error mt-2">{$exportError}</p>
            {/if}
        </div>
    </div>
    {/if}

    <!-- Developer (debug builds only) -->
    {#if $appInfo?.build_variant === "dev"}
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
                onclick={() => manager.handleRefreshFlags()}
                disabled={$isRefreshingFlags}
            >
                {$isRefreshingFlags ? "Refreshing…" : "Refresh Feature Flags"}
            </button>
            {#if $refreshFlagsMessage}
                <p class="text-xs text-slate-500 dark:text-navy-300 mt-2">{$refreshFlagsMessage}</p>
            {/if}
        </div>
    </div>
    {/if}
</div>
