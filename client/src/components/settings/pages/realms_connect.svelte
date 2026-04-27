<script lang="ts">
    import { onMount } from "svelte";
    import type { BedrockManager } from "../../../js/app/managers/bedrock/BedrockManager";
    import XboxLoginModal from "../bedrock/XboxLoginModal.svelte";
    import RealmCard from "../bedrock/RealmCard.svelte";

    interface Props {
        bedrockManager: BedrockManager;
    }

    let { bedrockManager }: Props = $props();

    const isEntitled = bedrockManager.isEntitled;
    const isAuthenticated = bedrockManager.isAuthenticated;
    const isRestoringAuth = bedrockManager.isRestoringAuth;
    const proxyRunning = bedrockManager.proxyRunning;
    const realmsRunning = bedrockManager.realmsRunning;
    const favorites = bedrockManager.favorites;
    const isLoadingRealms = bedrockManager.isLoadingRealms;
    const activeRealmId = bedrockManager.activeRealmId;
    const activeRealmName = bedrockManager.activeRealmName;
    const sortedRealms = bedrockManager.sortedRealms;
    const statusMessage = bedrockManager.statusMessage;
    const showLoginModal = bedrockManager.showLoginModal;

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
        {#if $proxyRunning}
            <div class="card p-4 border-l-4 border-warning">
                <div class="flex items-center gap-2">
                    <svg xmlns="http://www.w3.org/2000/svg" class="size-5 text-warning" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-2.5L13.732 4c-.77-.833-1.964-.833-2.732 0L4.082 16.5c-.77.833.192 2.5 1.732 2.5z"/>
                    </svg>
                    <p class="text-sm font-medium text-slate-700 dark:text-navy-100">Proxy session is active. Stop it before connecting to a Realm.</p>
                </div>
            </div>
        {/if}

        {#if !$isAuthenticated}
            <div class="card p-4 lg:p-6">
                <h2 class="font-medium tracking-wide text-slate-700 dark:text-navy-100 lg:text-base pb-2">
                    Xbox Live Authentication
                </h2>
                <p class="text-sm text-slate-500 dark:text-navy-300">
                    Sign in with your Xbox Live account to browse and connect to your Realms.
                </p>
                <button
                    class="btn mt-4 bg-success font-medium text-white hover:bg-success-focus"
                    onclick={() => bedrockManager.openLoginModal()}
                >
                    Sign in with Xbox Live
                </button>
            </div>
        {:else}
            {#if $realmsRunning}
                <div class="card p-4 border-l-4 border-success">
                    <div class="flex items-center justify-between">
                        <div>
                            <p class="text-sm font-medium text-slate-700 dark:text-navy-100">
                                Connected to {$activeRealmName}
                            </p>
                            <p class="text-xs text-slate-500 dark:text-navy-300">Realms session active</p>
                        </div>
                        <button
                            class="btn bg-error font-medium text-white hover:bg-error-focus"
                            onclick={() => bedrockManager.stopRealms()}
                        >
                            Disconnect
                        </button>
                    </div>
                </div>
            {/if}

            <div class="flex items-center justify-between">
                <h2 class="font-medium tracking-wide text-slate-700 dark:text-navy-100 lg:text-base">
                    Your Realms
                </h2>
                <div class="flex items-center gap-2">
                    <button
                        class="btn bg-slate-150 font-medium text-slate-800 hover:bg-slate-200
                               dark:bg-navy-500 dark:text-navy-100 dark:hover:bg-navy-450"
                        onclick={() => bedrockManager.loadRealms()}
                        disabled={$isLoadingRealms || $realmsRunning}
                    >
                        {$isLoadingRealms ? "Loading..." : "Refresh"}
                    </button>
                    {#if !$realmsRunning}
                        <button
                            class="btn text-xs+ text-slate-500 hover:text-error dark:text-navy-300 dark:hover:text-error"
                            onclick={() => bedrockManager.signOut()}
                        >
                            Sign Out
                        </button>
                    {/if}
                </div>
            </div>

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
                            isFavorite={$favorites.has(realm.id)}
                            isActive={$realmsRunning && $activeRealmId === realm.id}
                            disabled={$realmsRunning || $proxyRunning}
                            onConnect={() => bedrockManager.connectToRealm(realm)}
                            onToggleFavorite={() => bedrockManager.toggleFavorite(realm.id)}
                        />
                    {/each}
                </div>
            {/if}
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
