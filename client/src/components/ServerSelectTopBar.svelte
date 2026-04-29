<script lang="ts">
    interface Props {
        title?: string;
        subtitle?: string;
        isRefreshing: boolean;
        onRefreshAll: () => void;
        onAddServer: () => void;
    }

    let {
        title = "Choose Server",
        subtitle = "Pick a server to connect to, or add a new one.",
        isRefreshing,
        onRefreshAll,
        onAddServer,
    }: Props = $props();
</script>

<div class="flex flex-col gap-4 sm:flex-row sm:items-center sm:justify-between">
    <div>
        <h1 class="text-2xl font-semibold tracking-tight text-slate-800 dark:text-navy-50 sm:text-3xl">
            {title}
        </h1>
        <p class="mt-1 text-sm text-slate-500 dark:text-navy-200">
            {subtitle}
        </p>
    </div>
    <div class="flex items-center gap-2">
        <button
            class="btn font-medium text-slate-800 bg-slate-150 hover:bg-slate-200
                   dark:bg-navy-500 dark:text-navy-100 dark:hover:bg-navy-450
                   disabled:opacity-50 disabled:cursor-not-allowed"
            onclick={onRefreshAll}
            disabled={isRefreshing}
        >
            <svg xmlns="http://www.w3.org/2000/svg" class="size-4 mr-2 {isRefreshing ? 'animate-spin' : ''}" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15"/>
            </svg>
            {isRefreshing ? "Refreshing…" : "Refresh All"}
        </button>
        <button
            class="btn font-medium text-white bg-primary hover:bg-primary-focus
                   dark:bg-accent dark:hover:bg-accent-focus"
            onclick={onAddServer}
        >
            <svg xmlns="http://www.w3.org/2000/svg" class="size-4 mr-2" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2.5" d="M12 4v16m8-8H4"/>
            </svg>
            New Server
        </button>
    </div>
</div>
