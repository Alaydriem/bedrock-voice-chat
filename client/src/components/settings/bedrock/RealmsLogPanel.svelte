<script lang="ts">
    import { tick } from "svelte";
    import type { BedrockManager } from "../../../js/app/managers/bedrock/BedrockManager";

    interface Props {
        bedrockManager: BedrockManager;
    }

    let { bedrockManager }: Props = $props();

    const logs = bedrockManager.realmsLogs;
    const expanded = bedrockManager.logsExpanded;

    let scrollEl: HTMLDivElement | null = $state(null);
    let pinnedToBottom = $state(true);

    function onScroll(e: Event): void {
        const el = e.currentTarget as HTMLDivElement;
        pinnedToBottom = el.scrollHeight - el.scrollTop - el.clientHeight < 16;
    }

    $effect(() => {
        const _len = $logs.length;
        const _open = $expanded;
        if (!scrollEl || !pinnedToBottom) return;
        tick().then(() => {
            if (scrollEl) scrollEl.scrollTop = scrollEl.scrollHeight;
        });
    });

    function levelClass(level: string): string {
        switch (level) {
            case "ERROR": return "text-error";
            case "WARN": return "text-warning";
            case "INFO": return "text-slate-600 dark:text-navy-200";
            default: return "text-slate-400 dark:text-navy-300";
        }
    }

    function formatTimestamp(ms: bigint | number): string {
        const d = new Date(Number(ms));
        return d.toLocaleTimeString(undefined, { hour12: false }) + "." + String(d.getMilliseconds()).padStart(3, "0");
    }
</script>

<div class="card overflow-hidden">
    <div class="flex w-full items-center justify-between p-3 gap-3">
        <button
            type="button"
            class="flex flex-1 items-center gap-2 text-left hover:opacity-80 transition-opacity"
            onclick={() => bedrockManager.toggleLogs()}
        >
            <svg
                xmlns="http://www.w3.org/2000/svg"
                class="size-4 text-slate-500 dark:text-navy-300 transition-transform {$expanded ? 'rotate-90' : ''}"
                fill="none"
                viewBox="0 0 24 24"
                stroke="currentColor"
            >
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 5l7 7-7 7"/>
            </svg>
            <span class="text-sm font-medium text-slate-700 dark:text-navy-100">Realms Logs</span>
            <span class="text-xs text-slate-400 dark:text-navy-300">({$logs.length})</span>
        </button>
        {#if $expanded && $logs.length > 0}
            <button
                type="button"
                class="text-xs+ text-slate-500 hover:text-error dark:text-navy-300 dark:hover:text-error"
                onclick={() => bedrockManager.clearLogs()}
            >
                Clear
            </button>
        {/if}
    </div>

    {#if $expanded}
        <div
            bind:this={scrollEl}
            onscroll={onScroll}
            class="border-t border-slate-200 dark:border-navy-500 max-h-64 overflow-y-auto bg-slate-50 dark:bg-navy-800 font-mono text-tiny+"
        >
            {#if $logs.length === 0}
                <p class="p-4 text-center text-slate-400 dark:text-navy-300">No logs yet.</p>
            {:else}
                {#each $logs as entry}
                    <div class="px-3 py-0.5 flex gap-2">
                        <span class="text-slate-400 dark:text-navy-300 shrink-0">{formatTimestamp(entry.timestamp_ms)}</span>
                        <span class="shrink-0 font-semibold {levelClass(entry.level)}">{entry.level}</span>
                        <span class="break-all {levelClass(entry.level)}">{entry.message}</span>
                    </div>
                {/each}
            {/if}
        </div>
    {/if}
</div>
