<script lang="ts">
    import { onMount } from "svelte";
    import type { BedrockManager } from "../../../js/app/managers/bedrock/BedrockManager";
    import XboxLoginModal from "../bedrock/XboxLoginModal.svelte";

    interface Props {
        bedrockManager: BedrockManager;
    }

    let { bedrockManager }: Props = $props();

    const isEntitled = bedrockManager.isEntitled;
    const isAuthenticated = bedrockManager.isAuthenticated;
    const isRestoringAuth = bedrockManager.isRestoringAuth;
    const proxyRunning = bedrockManager.proxyRunning;
    const realmsRunning = bedrockManager.realmsRunning;
    const interfaces = bedrockManager.interfaces;
    const statusMessage = bedrockManager.statusMessage;
    const showLoginModal = bedrockManager.showLoginModal;
    const serverHost = bedrockManager.serverHost;
    const serverPort = bedrockManager.serverPort;
    const listenPort = bedrockManager.listenPort;
    const selectedInterface = bedrockManager.selectedInterface;
    const isProxyLoading = bedrockManager.isProxyLoading;
    const canStartProxy = bedrockManager.canStartProxy;

    let showAdvanced = $state(false);

    onMount(() => { bedrockManager.initialize(); });
</script>

<div class="grid grid-cols-1 gap-4 sm:gap-5 lg:gap-6 pt-4 md:pt-0">
    {#if $isRestoringAuth}
        <div class="card p-8 flex items-center justify-center gap-3">
            <div class="size-5 animate-spin rounded-full border-2 border-slate-300 border-t-primary dark:border-navy-400 dark:border-t-accent"></div>
            <span class="text-sm text-slate-500 dark:text-navy-300">Restoring session...</span>
        </div>
    {:else if !$isEntitled}
        <div class="card p-4 lg:p-6 border-l-4 border-warning">
            <p class="text-sm text-slate-700 dark:text-navy-200">Bedrock features require a purchase.</p>
            <button class="btn mt-3 bg-primary font-medium text-white hover:bg-primary-focus">
                Purchase Bedrock Add-on
            </button>
        </div>
    {:else}
        {#if $realmsRunning}
            <div class="card p-4 border-l-4 border-warning">
                <div class="flex items-center gap-2">
                    <svg xmlns="http://www.w3.org/2000/svg" class="size-5 text-warning" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-2.5L13.732 4c-.77-.833-1.964-.833-2.732 0L4.082 16.5c-.77.833.192 2.5 1.732 2.5z"/>
                    </svg>
                    <p class="text-sm font-medium text-slate-700 dark:text-navy-100">Realms session is active. Stop it before starting a proxy connection.</p>
                </div>
            </div>
        {/if}

        {#if !$isAuthenticated}
            <div class="card p-4 lg:p-6">
                <h2 class="font-medium tracking-wide text-slate-700 dark:text-navy-100 lg:text-base pb-2">
                    Xbox Live Authentication
                </h2>
                <p class="text-sm text-slate-500 dark:text-navy-300">
                    Sign in with your Xbox Live account to use Bedrock proxy features.
                </p>
                <button
                    class="btn mt-4 bg-success font-medium text-white hover:bg-success-focus"
                    onclick={() => bedrockManager.openLoginModal()}
                >
                    Sign in with Xbox Live
                </button>
            </div>
        {:else}
            <div class="card p-4 lg:p-6">
                <div class="my-1 flex items-start justify-between">
                    <div>
                        <h2 class="font-medium tracking-wide text-slate-700 dark:text-navy-100 lg:text-base pb-2">
                            Proxy Connection
                        </h2>
                        <p class="text-sm leading-6 text-slate-500 dark:text-navy-300 hidden md:block">
                            Connect to a Bedrock server through BVC's proxy to enable proximity voice chat.
                        </p>
                    </div>
                    {#if !$proxyRunning}
                        <button
                            class="btn text-xs+ text-slate-500 hover:text-error dark:text-navy-300 dark:hover:text-error"
                            onclick={() => bedrockManager.signOut()}
                        >
                            Sign Out
                        </button>
                    {/if}
                </div>

                <div class="mt-4 grid grid-cols-1 gap-4 sm:grid-cols-2 sm:gap-5 lg:gap-6">
                    <label class="block">
                        <span class="text-xs+ font-medium text-slate-700 dark:text-navy-100">Server Host</span>
                        <input
                            type="text"
                            class="form-input mt-1.5 w-full rounded-lg border border-slate-300 bg-transparent px-3 py-2
                                   hover:border-slate-400 focus:border-primary dark:border-navy-450 dark:bg-navy-700
                                   dark:hover:border-navy-400 dark:focus:border-accent"
                            value={$serverHost}
                            oninput={(e) => bedrockManager.setServerHost(e.currentTarget.value)}
                            placeholder="play.example.com"
                            disabled={$proxyRunning}
                        />
                    </label>
                    <label class="block">
                        <span class="text-xs+ font-medium text-slate-700 dark:text-navy-100">Server Port</span>
                        <input
                            type="number"
                            class="form-input mt-1.5 w-full rounded-lg border border-slate-300 bg-transparent px-3 py-2
                                   hover:border-slate-400 focus:border-primary dark:border-navy-450 dark:bg-navy-700
                                   dark:hover:border-navy-400 dark:focus:border-accent"
                            value={$serverPort}
                            oninput={(e) => bedrockManager.setServerPort(parseInt(e.currentTarget.value) || 19132)}
                            disabled={$proxyRunning}
                        />
                    </label>
                </div>

                <button
                    class="mt-4 flex items-center gap-1 text-xs+ font-medium text-primary hover:text-primary-focus dark:text-accent-light dark:hover:text-accent transition-colors"
                    onclick={() => { showAdvanced = !showAdvanced; }}
                >
                    <svg xmlns="http://www.w3.org/2000/svg" class="size-4 transition-transform {showAdvanced ? 'rotate-90' : ''}" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 5l7 7-7 7"/>
                    </svg>
                    {showAdvanced ? "Hide" : "View More"} Settings
                </button>

                {#if showAdvanced}
                    <div class="mt-4 grid grid-cols-1 gap-4 sm:grid-cols-2 sm:gap-5">
                        <label class="block">
                            <span class="text-xs+ font-medium text-slate-700 dark:text-navy-100">Listen Port</span>
                            <input
                                type="number"
                                class="form-input mt-1.5 w-full rounded-lg border border-slate-300 bg-transparent px-3 py-2
                                       hover:border-slate-400 focus:border-primary dark:border-navy-450 dark:bg-navy-700
                                       dark:hover:border-navy-400 dark:focus:border-accent"
                                value={$listenPort}
                                oninput={(e) => bedrockManager.setListenPort(parseInt(e.currentTarget.value) || 19137)}
                                disabled={$proxyRunning}
                            />
                            <span class="text-tiny+ text-slate-400 dark:text-navy-300">
                                Local port Minecraft connects to (default: 19137)
                            </span>
                        </label>
                        <label class="block">
                            <span class="text-xs+ font-medium text-slate-700 dark:text-navy-100">Network Interface</span>
                            <select
                                class="form-select mt-1.5 w-full rounded-lg border border-slate-300 bg-transparent px-3 py-2
                                       hover:border-slate-400 focus:border-primary dark:border-navy-450 dark:bg-navy-700
                                       dark:hover:border-navy-400 dark:focus:border-accent"
                                value={$selectedInterface}
                                onchange={(e) => bedrockManager.setSelectedInterface(e.currentTarget.value)}
                                disabled={$proxyRunning}
                            >
                                {#each $interfaces as iface}
                                    <option value={iface.ip}>{iface.name} ({iface.ip})</option>
                                {/each}
                            </select>
                            <span class="text-tiny+ text-slate-400 dark:text-navy-300">
                                Interface other players can reach
                            </span>
                        </label>
                    </div>
                {/if}

                <div class="mt-6 flex items-center gap-3">
                    {#if !$proxyRunning}
                        <button
                            class="btn bg-success font-medium text-white hover:bg-success-focus"
                            onclick={() => bedrockManager.startProxy()}
                            disabled={!$canStartProxy || $isProxyLoading}
                        >
                            {$isProxyLoading ? "Starting..." : "Start Proxy"}
                        </button>
                    {:else}
                        <button
                            class="btn bg-error font-medium text-white hover:bg-error-focus"
                            onclick={() => bedrockManager.stopProxy()}
                        >
                            Stop Proxy
                        </button>
                        <span class="badge rounded-full bg-success/10 text-success dark:bg-success/15">
                            Running
                        </span>
                    {/if}
                </div>
            </div>
        {/if}

        {#if $statusMessage}
            <div class="card p-4">
                <p class="text-sm {$statusMessage.startsWith('Error') ? 'text-error' : 'text-success'}">
                    {$statusMessage}
                </p>
            </div>
        {/if}
    {/if}
</div>

{#if $showLoginModal}
    <XboxLoginModal {bedrockManager} />
{/if}
