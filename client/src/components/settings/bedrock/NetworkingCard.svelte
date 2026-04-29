<script lang="ts">
    import type { BedrockManager } from "../../../js/app/managers/bedrock/BedrockManager";

    interface Props {
        bedrockManager: BedrockManager;
        showListenPort?: boolean;
    }

    let { bedrockManager, showListenPort = true }: Props = $props();

    const proxyRunning = bedrockManager.proxyRunning;
    const realmsRunning = bedrockManager.realmsRunning;
    const interfaces = bedrockManager.interfaces;
    const listenPort = bedrockManager.listenPort;
    const selectedInterface = bedrockManager.selectedInterface;

    let showAdvanced = $state(false);
</script>

<div class="card p-4 lg:p-6">
    <button
        class="flex items-center gap-1 text-xs+ font-medium text-primary hover:text-primary-focus dark:text-accent-light dark:hover:text-accent transition-colors"
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
                <span class="text-xs+ font-medium text-slate-700 dark:text-navy-100">Network Interface</span>
                <select
                    class="form-select mt-1.5 w-full rounded-lg border border-slate-300 bg-transparent px-3 py-2
                           hover:border-slate-400 focus:border-primary dark:border-navy-450 dark:bg-navy-700
                           dark:hover:border-navy-400 dark:focus:border-accent"
                    value={$selectedInterface}
                    onchange={(e) => bedrockManager.setSelectedInterface(e.currentTarget.value)}
                    disabled={$proxyRunning || $realmsRunning}
                >
                    {#each $interfaces as iface}
                        <option value={iface.ip}>{iface.name} ({iface.ip})</option>
                    {/each}
                </select>
                <span class="text-tiny+ text-slate-400 dark:text-navy-300">
                    Interface other players can reach
                </span>
            </label>
            {#if showListenPort}
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
            {/if}
        </div>
    {/if}
</div>
