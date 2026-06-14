<script lang="ts">
    import type { BedrockManager } from "../../../js/app/managers/bedrock/BedrockManager";
    import type { ProxyServerEntry } from "../../../js/app/managers/bedrock/ProxyServerEntry";
    import PageShell from "../bedrock/PageShell.svelte";
    import ProxyServerCard from "../bedrock/ProxyServerCard.svelte";
    import ProxyServerModal from "../bedrock/ProxyServerModal.svelte";

    interface Props {
        bedrockManager: BedrockManager;
    }

    let { bedrockManager }: Props = $props();

    const proxyRunning = bedrockManager.proxyRunning;
    const realmsRunning = bedrockManager.realmsRunning;
    const sortedProxyServers = bedrockManager.sortedProxyServers;
    const proxyFavorites = bedrockManager.proxyFavorites;
    const activeProxyId = bedrockManager.activeProxyId;

    type ModalState = { mode: "add" } | { mode: "edit"; entry: ProxyServerEntry } | null;
    let modalState = $state<ModalState>(null);

    function confirmDelete(entry: ProxyServerEntry) {
        if (confirm(`Delete "${entry.name}"?`)) {
            bedrockManager.deleteProxyServer(entry.id);
        }
    }
</script>

<PageShell
    {bedrockManager}
    title="Saved Proxy Servers"
    signedOutDescription="Sign in with your Xbox Live account to use Bedrock proxy features."
    showListenPort={true}
>
    {#snippet extraActions()}
        <button
            class="btn bg-primary font-medium text-white hover:bg-primary-focus
                   dark:bg-accent dark:hover:bg-accent-focus
                   disabled:opacity-50 disabled:cursor-not-allowed"
            onclick={() => { modalState = { mode: "add" }; }}
            disabled={$proxyRunning || $realmsRunning}
        >
            Add Server
        </button>
    {/snippet}

    {#snippet body()}
        {#if $sortedProxyServers.length === 0}
            <div class="card p-8 text-center">
                <svg xmlns="http://www.w3.org/2000/svg" class="size-12 mx-auto text-slate-300 dark:text-navy-400" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M5 12h14M5 12a2 2 0 01-2-2V6a2 2 0 012-2h14a2 2 0 012 2v4a2 2 0 01-2 2M5 12a2 2 0 00-2 2v4a2 2 0 002 2h14a2 2 0 002-2v-4a2 2 0 00-2-2m-2-4h.01M17 16h.01"/>
                </svg>
                <p class="mt-3 text-sm text-slate-500 dark:text-navy-300">
                    No saved proxy servers. Click Add Server to get started.
                </p>
            </div>
        {:else}
            <div class="grid grid-cols-1 gap-4 sm:grid-cols-2 sm:gap-5 lg:grid-cols-3 lg:gap-6">
                {#each $sortedProxyServers as entry (entry.id)}
                    <ProxyServerCard
                        {entry}
                        isFavorite={$proxyFavorites.has(entry.id)}
                        isActive={$proxyRunning && $activeProxyId === entry.id}
                        disabled={$proxyRunning || $realmsRunning}
                        onConnect={() => bedrockManager.connectToProxyServer(entry)}
                        onDisconnect={() => bedrockManager.stopProxy()}
                        onToggleFavorite={() => bedrockManager.toggleProxyFavorite(entry.id)}
                        onEdit={() => { modalState = { mode: "edit", entry }; }}
                        onDelete={() => confirmDelete(entry)}
                    />
                {/each}
            </div>
        {/if}
    {/snippet}
</PageShell>

{#if modalState}
    <ProxyServerModal
        {bedrockManager}
        initial={modalState.mode === "edit" ? modalState.entry : undefined}
        onClose={() => { modalState = null; }}
    />
{/if}
