<script lang="ts">
    import { onMount } from "svelte";
    import type { BedrockManager } from "../../../js/app/managers/bedrock/BedrockManager";
    import PageShell from "../bedrock/PageShell.svelte";
    import RealmCard from "../bedrock/RealmCard.svelte";
    import RealmsUpsell from "../bedrock/RealmsUpsell.svelte";
    import ServerCapabilityNotice from "../bedrock/ServerCapabilityNotice.svelte";

    interface Props {
        bedrockManager: BedrockManager;
    }

    let { bedrockManager }: Props = $props();

    const capabilityStatus = bedrockManager.capability.status;
    const capabilityServerHost = bedrockManager.capability.serverHost;
    const capabilityChecking = bedrockManager.capability.isChecking;
    const gateStatus = bedrockManager.gateStatus;
    const proxyRunning = bedrockManager.proxyRunning;
    const realmsRunning = bedrockManager.realmsRunning;
    const favorites = bedrockManager.favorites;
    const isLoadingRealms = bedrockManager.isLoadingRealms;
    const activeRealmId = bedrockManager.activeRealmId;
    const sortedRealms = bedrockManager.sortedRealms;

    onMount(() => {
        bedrockManager.initializeRealmsAccess();
    });
</script>

{#if $capabilityStatus !== "enabled"}
    <ServerCapabilityNotice status={$capabilityStatus} serverHost={$capabilityServerHost} isChecking={$capabilityChecking} onRetry={() => bedrockManager.capability.refresh()} />
{:else if $gateStatus === null}
    <div class="grid grid-cols-1 gap-4 sm:gap-5 lg:gap-6 pt-4 md:pt-0">
        <div class="card flex flex-col items-center justify-center gap-5 px-6 py-16 text-center">
            <div class="relative flex size-16 items-center justify-center">
                <div class="absolute inset-0 animate-spin rounded-full border-2 border-slate-200 border-t-primary dark:border-navy-500 dark:border-t-accent-light"></div>
                <svg xmlns="http://www.w3.org/2000/svg" class="size-7 text-primary dark:text-accent-light" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.8" d="M20 7l-8-4-8 4m16 0l-8 4m8-4v10l-8 4m0-10L4 7m8 4v10M4 7v10l8 4"/>
                </svg>
            </div>
            <div class="space-y-1.5">
                <h2 class="text-base font-semibold text-slate-700 dark:text-navy-100 lg:text-lg">
                    Checking your access
                </h2>
                <p class="mx-auto max-w-sm text-sm leading-relaxed text-slate-500 dark:text-navy-300">
                    Confirming your Realms Connect subscription. This only takes a moment.
                </p>
            </div>
        </div>
    </div>
{:else if $gateStatus.status === "allowed"}
    <PageShell
        {bedrockManager}
        title="Your Realms"
        signedOutDescription="Sign in with your Xbox Live account to browse and connect to your Realms."
        showListenPort={false}
    >
        {#snippet body()}
            {#if $isLoadingRealms}
                <div class="grid grid-cols-1 gap-4 sm:grid-cols-2 sm:gap-5 lg:grid-cols-3 lg:gap-6">
                    {#each [1, 2, 3] as _}
                        <div class="card animate-pulse rounded-2xl">
                            <div class="h-36 rounded-t-2xl bg-slate-200 dark:bg-navy-600"></div>
                            <div class="p-3">
                                <div class="h-4 w-24 rounded bg-slate-200 dark:bg-navy-600"></div>
                            </div>
                        </div>
                    {/each}
                </div>
            {:else if $sortedRealms.length === 0}
                <div class="card p-8 text-center">
                    <svg xmlns="http://www.w3.org/2000/svg" class="size-12 mx-auto text-slate-300 dark:text-navy-400" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M20 7l-8-4-8 4m16 0l-8 4m8-4v10l-8 4m0-10L4 7m8 4v10M4 7v10l8 4"/>
                    </svg>
                    <p class="mt-3 text-sm text-slate-500 dark:text-navy-300">
                        No Realms found. You must be a member of at least one Realm.
                    </p>
                </div>
            {:else}
                <div class="grid grid-cols-1 gap-4 sm:grid-cols-2 sm:gap-5 lg:grid-cols-3 lg:gap-6">
                    {#each $sortedRealms as realm (realm.id)}
                        <RealmCard
                            {realm}
                            isFavorite={$favorites.has(String(realm.id))}
                            isActive={$realmsRunning && $activeRealmId === realm.id}
                            disabled={$realmsRunning || $proxyRunning}
                            onConnect={() => bedrockManager.connectToRealm(realm)}
                            onDisconnect={() => bedrockManager.stopRealms()}
                            onToggleFavorite={() => bedrockManager.toggleFavorite(realm.id)}
                        />
                    {/each}
                </div>
            {/if}
        {/snippet}
    </PageShell>
{:else}
    <RealmsUpsell {bedrockManager} />
{/if}
