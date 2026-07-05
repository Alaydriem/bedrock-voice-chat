<script lang="ts">
    import { onMount, onDestroy } from "svelte";
    import { KeybindsManager } from "../../../js/app/managers/settings/KeybindsManager";

    const manager = new KeybindsManager();

    const isReady = manager.isReady;
    const config = manager.config;
    const editingId = manager.editingId;
    const capturedCombo = manager.capturedCombo;
    const conflictError = manager.conflictError;

    onMount(() => {
        manager.initialize();
    });

    onDestroy(() => {
        manager.destroy();
    });
</script>

<div class="grid grid-cols-1 gap-4 sm:gap-5 lg:gap-6 pt-4 md:pt-0">
    <div class="card px-5 pb-4 sm:px-5">
        <div class="my-3 flex flex-col">
            <h2 class="font-medium tracking-wide text-slate-700 line-clamp-1 dark:text-navy-100 lg:text-base pb-2">
                Keyboard Shortcuts
            </h2>
            <p class="text-sm leading-6 hidden md:block">
                Configure global keyboard shortcuts for voice controls. These work even when BVC is not focused.
            </p>
        </div>

        {#if $isReady}
        <div class="space-y-3 mt-2">
            {#each manager.rows as row}
                {@const isEditing = $editingId === row.id}
                {@const isHiddenInMode =
                    (row.id === "toggleMute" && $config.voiceMode === "pushToTalk") ||
                    (row.id === "pushToTalk" && $config.voiceMode === "openMic")}
                <div class="flex items-center justify-between py-3 px-4 rounded-lg transition-colors
                    {isEditing ? 'bg-primary/10 dark:bg-accent/15 ring-1 ring-primary/30 dark:ring-accent/30' : 'hover:bg-slate-50 dark:hover:bg-navy-600'}
                    {isHiddenInMode ? 'opacity-40' : ''}">
                    <div class="flex-1">
                        <span class="text-sm font-medium text-slate-700 dark:text-navy-100">
                            {row.label}
                        </span>
                        {#if isHiddenInMode}
                            <span class="ml-2 text-xs text-slate-400 dark:text-navy-300">
                                (not active in {$config.voiceMode === "pushToTalk" ? "Push to Talk" : "Open Mic"} mode)
                            </span>
                        {/if}
                    </div>
                    <div class="flex items-center space-x-2">
                        {#if isEditing}
                            <span class="text-sm text-primary dark:text-accent-light animate-pulse">
                                {$capturedCombo ? manager.displayCombo($capturedCombo) : "Press a key combo..."}
                            </span>
                            {#if $conflictError}
                                <span class="text-xs text-error">{$conflictError}</span>
                            {/if}
                            <button
                                class="btn px-2 py-1 text-xs rounded bg-slate-200 hover:bg-slate-300 dark:bg-navy-500 dark:hover:bg-navy-400 text-slate-600 dark:text-navy-100"
                                onclick={() => manager.cancelEditing()}
                            >
                                Cancel
                            </button>
                        {:else}
                            <kbd class="px-2 py-1 text-sm font-mono bg-slate-100 dark:bg-navy-600 text-slate-700 dark:text-navy-100 rounded border border-slate-200 dark:border-navy-500">
                                {manager.displayCombo($config[row.id] as string)}
                            </kbd>
                            <button
                                class="btn px-2 py-1 text-xs rounded bg-primary/10 hover:bg-primary/20 dark:bg-accent/15 dark:hover:bg-accent/25 text-primary dark:text-accent-light"
                                onclick={() => manager.startEditing(row.id)}
                                disabled={isHiddenInMode}
                            >
                                Edit
                            </button>
                            <button
                                class="btn px-2 py-1 text-xs rounded bg-slate-200 hover:bg-slate-300 dark:bg-navy-500 dark:hover:bg-navy-400 text-slate-600 dark:text-navy-100"
                                onclick={() => manager.resetBinding(row.id)}
                                disabled={isHiddenInMode}
                            >
                                Reset
                            </button>
                        {/if}
                    </div>
                </div>
            {/each}
        </div>

        <div class="my-4 h-px bg-slate-200 dark:bg-navy-500"></div>

        <div class="flex justify-end">
            <button
                class="btn px-4 py-2 text-sm rounded-lg bg-slate-200 hover:bg-slate-300 dark:bg-navy-500 dark:hover:bg-navy-400 text-slate-700 dark:text-navy-100"
                onclick={() => manager.resetAll()}
            >
                Reset All Keybinds
            </button>
        </div>
        {/if}
    </div>
</div>
