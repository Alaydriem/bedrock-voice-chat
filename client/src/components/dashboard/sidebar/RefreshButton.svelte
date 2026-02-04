<script lang="ts">
    import { info, error as logError } from '@tauri-apps/plugin-log';
    import { onMount, onDestroy } from 'svelte';
    import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow';

    export let disabled: boolean = false;

    let isRefreshing = false;
    let unlisten: (() => void) | undefined;

    const handleRefresh = async () => {
        try {
            isRefreshing = true;
            info('Refreshing audio engine...');

            if (window.App) {
                await window.App.shutdown();
            }
            window.location.reload();
        } catch (error) {
            logError(`Failed to refresh audio engine: ${error}`);
            isRefreshing = false;
        }
    };

    onMount(async () => {
        const appWebview = getCurrentWebviewWindow();
        unlisten = await appWebview.listen('trigger_refresh', () => {
            info('Received trigger_refresh event, initiating refresh...');
            handleRefresh();
        });
    });

    onDestroy(() => {
        if (unlisten) {
            unlisten();
        }
    });
</script>

<button
    class="pl-2 pr-2 btn size-8 rounded-full p-0 hover:bg-slate-300/20 focus:bg-slate-300/20 active:bg-slate-300/25 dark:hover:bg-navy-300/20 dark:focus:bg-navy-300/20 dark:active:bg-navy-300/25"
    class:animate-spin={isRefreshing}
    on:click={handleRefresh}
    {disabled}
    data-tooltip="Refresh"
    aria-label="Refresh"
    title="Reload Audio Engine"
>
    <i class="fa-solid fa-arrows-rotate"></i>
</button>
