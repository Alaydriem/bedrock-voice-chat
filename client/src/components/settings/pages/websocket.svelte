<script lang="ts">
    import { onMount } from "svelte";
    import { WebSocketSettingsManager } from "../../../js/app/managers/settings/WebSocketSettingsManager";

    const manager = new WebSocketSettingsManager();

    const isReady = manager.isReady;
    const localhostOnly = manager.localhostOnly;
    const websocketPort = manager.websocketPort;
    const authKey = manager.authKey;
    const isRunning = manager.isRunning;

    onMount(() => {
        manager.initialize();
    });
</script>

<div class="grid grid-cols-1 gap-4 sm:gap-5 lg:gap-6 pt-4 md:pt-0">
    <div class="card px-5 pb-4 sm:px-5">
        <div class="my-3 flex flex-col">
            <h2 class="font-medium tracking-wide text-slate-700 dark:text-navy-100 lg:text-base pb-2">
                WebSocket Server
            </h2>
            <p class="text-sm leading-6 hidden md:block">
                Enable remote control via WebSocket for Stream Deck and other integrations
            </p>
        </div>

        {#if $isReady}
        <div class="space-y-4">
            <div class="flex items-center justify-between">
                <div>
                    <span class="text-sm font-medium">Restrict to Localhost</span>
                    <p class="text-xs text-slate-500 dark:text-navy-300 mt-0.5">
                        {$localhostOnly ? "127.0.0.1 (localhost only)" : "0.0.0.0 (all interfaces)"}
                    </p>
                </div>
                <label class="inline-flex items-center space-x-2 cursor-pointer">
                    <input
                        type="checkbox"
                        checked={$localhostOnly}
                        onchange={() => manager.handleLocalhostToggle()}
                        class="form-switch h-5 w-10 rounded-full bg-slate-300 before:rounded-full before:bg-slate-50
                               checked:bg-primary checked:before:bg-white dark:bg-navy-900 dark:before:bg-navy-300
                               dark:checked:bg-accent dark:checked:before:bg-white"
                    />
                </label>
            </div>

            <label class="block">
                <span class="text-sm font-medium">Port</span>
                <input
                    type="text"
                    value={$websocketPort}
                    onchange={(e) => manager.handlePortChange((e.target as HTMLInputElement).value)}
                    class="form-input mt-1.5 w-full rounded-lg border border-slate-300 bg-white px-3 py-2
                           hover:border-slate-400 focus:border-primary dark:border-navy-450 dark:bg-navy-700"
                    placeholder="9595"
                />
            </label>

            <label class="block">
                <span class="text-sm font-medium">Authentication Key</span>
                <div class="flex gap-2 mt-1.5">
                    <input
                        type="text"
                        value={$authKey}
                        onchange={(e) => manager.handleKeyChange((e.target as HTMLInputElement).value)}
                        class="form-input flex-1 rounded-lg border border-slate-300 bg-white px-3 py-2
                               hover:border-slate-400 focus:border-primary dark:border-navy-450 dark:bg-navy-700"
                        placeholder="Enter authentication key"
                    />
                    <button
                        class="btn bg-primary font-medium text-white hover:bg-primary-focus
                               dark:bg-accent dark:hover:bg-accent-focus"
                        onclick={() => manager.handleGenerateKey()}
                    >
                        Generate
                    </button>
                </div>
            </label>

            <div class="my-4 h-px bg-slate-200 dark:bg-navy-500"></div>

            <div class="flex items-center justify-between">
                <div class="flex items-center gap-2">
                    <span class="text-sm font-medium">Enable WebSocket Server</span>
                    {#if $isRunning}
                        <span class="badge bg-success text-white">Running</span>
                    {:else}
                        <span class="badge bg-slate-300 text-slate-700 dark:bg-navy-500 text-warning">Stopped</span>
                    {/if}
                </div>
                <label class="inline-flex items-center space-x-2 cursor-pointer">
                    <input
                        type="checkbox"
                        checked={$isRunning}
                        onchange={() => manager.handleToggleServer()}
                        class="form-switch h-5 w-10 rounded-full bg-slate-300 before:rounded-full before:bg-slate-50
                               checked:bg-primary checked:before:bg-white dark:bg-navy-900 dark:before:bg-navy-300
                               dark:checked:bg-accent dark:checked:before:bg-white"
                    />
                </label>
            </div>
        </div>
        {/if}
    </div>
</div>
