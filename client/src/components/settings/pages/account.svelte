<script lang="ts">
    import { onMount } from "svelte";
    import { AccountManager } from "../../../js/app/managers/settings/AccountManager";

    const manager = new AccountManager();

    const gamertag = manager.gamertag;
    const gamerpic = manager.gamerpic;
    const minecraftUsername = manager.minecraftUsername;
    const isLinking = manager.isLinking;
    const linkError = manager.linkError;
    const isReady = manager.isReady;
    const isDesktop = manager.isDesktop;
    const activeGame = manager.activeGame;

    onMount(() => {
        manager.initialize();
    });
</script>

<div class="grid grid-cols-1 gap-4 sm:gap-5 lg:gap-6 pt-4 md:pt-0">
    <!-- Xbox Account -->
    <div class="card px-5 pb-4 sm:px-5">
        <div class="my-3 flex flex-col">
            <h2 class="font-medium tracking-wide text-slate-700 dark:text-navy-100 lg:text-base pb-2">
                {$activeGame === "hytale" ? "Hytale Account" : "Xbox Account"}
            </h2>
            <p class="text-sm leading-6 hidden md:block">
                {$activeGame === "hytale"
                    ? "Your Hytale identity used for voice chat authentication."
                    : "Your Xbox Live identity used for voice chat authentication."}
            </p>
        </div>

        {#if $isReady}
        <div class="flex items-center space-x-4 mt-2 py-3 px-3 rounded-lg">
            {#if $gamerpic}
            <img src={$gamerpic} alt="Gamerpic" class="size-12 rounded-full" />
            {:else}
            <div class="size-12 rounded-full bg-slate-200 dark:bg-navy-500 flex items-center justify-center">
                <svg xmlns="http://www.w3.org/2000/svg" class="size-6 text-slate-400" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M16 7a4 4 0 11-8 0 4 4 0 018 0zM12 14a7 7 0 00-7 7h14a7 7 0 00-7-7z"/>
                </svg>
            </div>
            {/if}
            <div>
                <span class="text-sm font-medium text-slate-700 dark:text-navy-100">{$gamertag || "Unknown"}</span>
                <p class="text-xs text-slate-500 dark:text-navy-300 mt-0.5">{$activeGame === "hytale" ? "Hytale Account" : "Xbox Gamertag"}</p>
            </div>
        </div>
        {/if}
    </div>

    {#if $activeGame === "minecraft"}
    <!-- Java Identity (Desktop only) -->
    <div class="card px-5 pb-4 sm:px-5">
        <div class="my-3 flex flex-col">
            <h2 class="font-medium tracking-wide text-slate-700 dark:text-navy-100 lg:text-base pb-2">
                Java Identity
            </h2>
            <p class="text-sm leading-6">
                Link your Minecraft Java Edition username for cross-platform voice chat on Geyser servers.
            </p>
        </div>

        {#if $isReady}
        <div class="space-y-3 mt-2">
            {#if $minecraftUsername}
            <div class="flex items-center justify-between py-2 px-3 rounded-lg bg-slate-50 dark:bg-navy-600">
                <div>
                    <span class="text-sm font-medium text-slate-700 dark:text-navy-100">{$minecraftUsername}</span>
                    <p class="text-xs text-slate-500 dark:text-navy-300 mt-0.5">Minecraft Java Username</p>
                </div>
                <span class="badge bg-success/10 text-success dark:bg-success/15">Linked</span>
            </div>
            {:else}
            <div class="flex items-center justify-between py-2 px-3 rounded-lg bg-slate-50 dark:bg-navy-600">
                <div>
                    <span class="text-sm text-slate-500 dark:text-navy-300">No Java identity linked</span>
                    <p class="text-xs text-slate-400 dark:text-navy-400 mt-0.5">Required for Geyser/Floodgate servers</p>
                </div>
                <span class="badge bg-warning/10 text-warning dark:bg-warning/15">Not linked</span>
            </div>
            {/if}

            {#if $isDesktop}
            <button
                class="btn bg-primary font-medium text-white hover:bg-primary-focus dark:bg-accent dark:hover:bg-accent-focus"
                onclick={() => manager.handleLinkJavaIdentity()}
                disabled={$isLinking}
            >
                {#if $isLinking}
                    <svg class="animate-spin -ml-1 mr-2 h-4 w-4 text-white" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24">
                        <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
                        <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4z"></path>
                    </svg>
                    Linking...
                {:else}
                    {$minecraftUsername ? "Re-link" : "Link Java Identity"}
                {/if}
            </button>
            {:else}
            <p class="text-xs text-slate-500 dark:text-navy-300">
                Java identity linking is available on the desktop app.
            </p>
            {/if}

            {#if $linkError}
                <p class="text-xs text-error mt-1">{$linkError}</p>
            {/if}
        </div>
        {/if}
    </div>
    {/if}
</div>
