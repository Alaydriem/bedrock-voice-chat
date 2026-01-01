<script lang="ts">
    import { onMount, onDestroy } from "svelte";
    import { invoke } from "@tauri-apps/api/core";
    import { Store } from '@tauri-apps/plugin-store';

    interface WebSocketConfig {
        enabled: boolean;
        host: string;
        port: number;
        key: string;
    }

    let store: Store | undefined = $state(undefined);
    let isReady = $state(false);

    // Settings state
    let websocketHost = $state("127.0.0.1");
    let websocketPort = $state("9595");
    let encryptionKey = $state("");
    let isRunning = $state(false);

    onMount(async () => {
        store = await Store.load("store.json", { autoSave: false });

        // Load saved config from single key
        const config = await store.get<WebSocketConfig>("websocket_server");
        if (config) {
            websocketHost = config.host || "127.0.0.1";
            websocketPort = config.port?.toString() || "9595";
            encryptionKey = config.key || "";
        }

        // Check status
        try {
            isRunning = await invoke('is_websocket_running');
        } catch (e) {
            console.error(e);
        }

        isReady = true;
    });

    async function saveConfig(enabled: boolean) {
        const config: WebSocketConfig = {
            enabled,
            host: websocketHost,
            port: parseInt(websocketPort),
            key: encryptionKey
        };
        await store?.set("websocket_server", config);
        await store?.save();

        // Update the manager's config
        await invoke('update_websocket_config', { config });
    }

    async function handleHostChange(event: Event) {
        websocketHost = (event.target as HTMLInputElement).value;
        await saveConfig(isRunning);
    }

    async function handlePortChange(event: Event) {
        websocketPort = (event.target as HTMLInputElement).value;
        await saveConfig(isRunning);
    }

    async function handleKeyChange(event: Event) {
        encryptionKey = (event.target as HTMLInputElement).value;
        await saveConfig(isRunning);
    }

    async function handleGenerateKey() {
        try {
            encryptionKey = await invoke<string>('generate_encryption_key');
            await saveConfig(isRunning);
        } catch (e) {
            console.error(e);
        }
    }

    async function handleToggleServer() {
        if (isRunning) {
            await stopServer();
        } else {
            await startServer();
        }
    }

    async function startServer() {
        try {
            // Save config with enabled=true
            await saveConfig(true);

            // Start the server
            await invoke('start_websocket_server');
            isRunning = true;
        } catch (e) {
            console.error(e);
        }
    }

    async function stopServer() {
        try {
            await invoke('stop_websocket_server');
            isRunning = false;

            // Save config with enabled=false
            await saveConfig(false);
        } catch (e) {
            console.error(e);
        }
    }
</script>

<div class="grid grid-cols-1 gap-4 sm:gap-5 lg:gap-6 pt-4 md:pt-0">
    <div class="card px-5 pb-4 sm:px-5">
        <div class="my-3 flex h-8 flex-col">
            <h2 class="font-medium tracking-wide text-slate-700 line-clamp-1 dark:text-navy-100 lg:text-base pb-2">
                WebSocket Server
            </h2>
            <p class="text-sm leading-6 hidden md:block">
                Enable remote control via WebSocket for Stream Deck and other integrations
            </p>
        </div>

        {#if isReady}
        <div class="space-y-4">
            <label class="block">
                <span class="text-sm font-medium">Host</span>
                <input
                    type="text"
                    value={websocketHost}
                    onchange={handleHostChange}
                    class="form-input mt-1.5 w-full rounded-lg border border-slate-300 bg-white px-3 py-2
                           hover:border-slate-400 focus:border-primary dark:border-navy-450 dark:bg-navy-700"
                    placeholder="127.0.0.1"
                />
            </label>

            <label class="block">
                <span class="text-sm font-medium">Port</span>
                <input
                    type="text"
                    value={websocketPort}
                    onchange={handlePortChange}
                    class="form-input mt-1.5 w-full rounded-lg border border-slate-300 bg-white px-3 py-2
                           hover:border-slate-400 focus:border-primary dark:border-navy-450 dark:bg-navy-700"
                    placeholder="9595"
                />
            </label>

            <label class="block">
                <span class="text-sm font-medium">Encryption Key</span>
                <div class="flex gap-2 mt-1.5">
                    <input
                        type="text"
                        value={encryptionKey}
                        onchange={handleKeyChange}
                        class="form-input flex-1 rounded-lg border border-slate-300 bg-white px-3 py-2
                               hover:border-slate-400 focus:border-primary dark:border-navy-450 dark:bg-navy-700"
                        placeholder="Enter encryption key"
                    />
                    <button
                        class="btn bg-primary font-medium text-white hover:bg-primary-focus
                               dark:bg-accent dark:hover:bg-accent-focus"
                        on:click={handleGenerateKey}
                    >
                        Generate
                    </button>
                </div>
            </label>

            <div class="my-4 h-px bg-slate-200 dark:bg-navy-500"></div>

            <div class="flex items-center justify-between">
                <div class="flex items-center gap-2">
                    <span class="text-sm font-medium">Server Status:</span>
                    {#if isRunning}
                        <span class="badge bg-success text-white">Running</span>
                    {:else}
                        <span class="badge bg-slate-300 text-slate-700 dark:bg-navy-500">Stopped</span>
                    {/if}
                </div>
                <button
                    class="btn {isRunning ? 'bg-error hover:bg-error-focus' : 'bg-success hover:bg-success-focus'} font-medium text-white
                           dark:{isRunning ? 'bg-error dark:hover:bg-error-focus' : 'bg-success dark:hover:bg-success-focus'}"
                    on:click={handleToggleServer}
                >
                    {isRunning ? 'Stop Server' : 'Start Server'}
                </button>
            </div>
        </div>
        {/if}
    </div>
</div>
