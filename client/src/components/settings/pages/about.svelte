<script lang="ts">
    import { onMount, onDestroy } from "svelte";
    import { invoke } from "@tauri-apps/api/core";
    import { openUrl } from "@tauri-apps/plugin-opener";
    import { error, info, warn } from "@tauri-apps/plugin-log";

    interface AppInfo {
        app_version: string;
        protocol_version: string;
        build_commit: string;
        build_variant: string;
    }

    interface AboutLink {
        url: string;
        title: string;
        description: string;
        icon: string;
    }

    const links: AboutLink[] = [
        {
            url: "https://github.com/alaydriem/bedrock-voice-chat/issues",
            title: "Report a Bug",
            description: "Open a bug report on GitHub",
            icon: `<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-2.5L13.732 4c-.77-.833-1.964-.833-2.732 0L4.082 16.5c-.77.833.192 2.5 1.732 2.5z"/>`
        },
        {
            url: "https://discord.gg/WGXy5kBP9E",
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
    let isExporting = $state(false);
    let exportError = $state("");

    let variantClickCount = $state(0);
    let variantClickTimer: ReturnType<typeof setTimeout> | null = null;

    function handleVariantClick() {
        variantClickCount++;

        if (variantClickTimer) clearTimeout(variantClickTimer);
        variantClickTimer = setTimeout(() => { variantClickCount = 0; }, 2000);

        switch (variantClickCount) {
            case 3: error("debug trigger: error (click 3)"); break;
            case 4: info("debug trigger: info (click 4)"); break;
            case 5: warn("debug trigger: warn (click 5)"); break;
            case 6:
                error("debug trigger: error (click 6)");
                variantClickCount = 0;
                break;
        }
    }

    onDestroy(() => {
        if (variantClickTimer) clearTimeout(variantClickTimer);
    });

    async function handleExportLogs() {
        isExporting = true;
        exportError = "";
        try {
            await invoke<boolean>("export_logs");
        } catch (e) {
            exportError = String(e);
            console.error("Failed to export logs:", e);
        } finally {
            isExporting = false;
        }
    }

    onMount(async () => {
        try {
            appInfo = await invoke<AppInfo>("get_app_info");
        } catch (e) {
            console.error("Failed to get app info:", e);
        }
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

    <!-- Diagnostics -->
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
</div>
