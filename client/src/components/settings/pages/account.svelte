<script lang="ts">
    import { onMount } from "svelte";
    import { Store } from "@tauri-apps/plugin-store";
    import { invoke } from "@tauri-apps/api/core";
    import { info, error as logError } from "@tauri-apps/plugin-log";
    import { platform } from "@tauri-apps/plugin-os";
    import Analytics from "../../../js/app/analytics";
    import type { LinkJavaIdentityResponse } from "../../../js/bindings/LinkJavaIdentityResponse";
    import type { Game } from "../../../js/bindings/Game";

    let gamertag = $state("");
    let gamerpic = $state("");
    let minecraftUsername = $state<string | null>(null);
    let isLinking = $state(false);
    let linkError = $state("");
    let isReady = $state(false);
    let isDesktop = $state(false);
    let activeGame = $state<Game>("minecraft");

    async function loadAccountInfo() {
        try {
            const os = platform();
            isDesktop = os === "windows" || os === "macos" || os === "linux";

            const store = await Store.load("store.json", { autoSave: false });
            const currentServer = await store.get<string>("current_server");

            if (!currentServer) return;

            const game = await store.get<string>("active_game");
            activeGame = (game === "hytale") ? "hytale" : "minecraft";

            gamertag = await invoke<string>("get_credential", { server: currentServer, key: "gamertag" }).catch(() => "");
            gamerpic = await invoke<string>("get_credential", { server: currentServer, key: "gamerpic" }).catch(() => "");

            try {
                const raw = await invoke<string>("get_credential", { server: currentServer, key: "minecraft_username" });
                minecraftUsername = (!raw || raw === "null" || raw === "") ? null : raw;
            } catch {
                minecraftUsername = null;
            }
        } catch (e) {
            logError(`Failed to load account info: ${e}`);
        }
        isReady = true;
    }

    async function handleLinkJavaIdentity() {
        isLinking = true;
        linkError = "";

        try {
            const store = await Store.load("store.json", { autoSave: false });
            const currentServer = await store.get<string>("current_server");

            if (!currentServer) {
                linkError = "Not connected to a server.";
                isLinking = false;
                return;
            }

            const response = await invoke("link_java_identity", {
                gamertag: gamertag,
            }) as LinkJavaIdentityResponse;

            if (response.minecraft_username) {
                minecraftUsername = response.minecraft_username;

                await invoke("set_credential", {
                    server: currentServer,
                    key: "minecraft_username",
                    value: response.minecraft_username
                });

                info(`Linked Java identity: ${response.minecraft_username}`);
                Analytics.track("JavaIdentityLinked");
            } else {
                linkError = "Could not retrieve Java username.";
            }
        } catch (e) {
            logError(`Failed to link Java identity: ${e}`);
            const errorStr = String(e);
            if (errorStr.includes("closed without completing")) {
                linkError = "";
            } else {
                linkError = "Failed to link Java identity.";
            }
        }

        isLinking = false;
    }

    onMount(() => {
        loadAccountInfo();
    });
</script>

<div class="grid grid-cols-1 gap-4 sm:gap-5 lg:gap-6 pt-4 md:pt-0">
    <!-- Xbox Account -->
    <div class="card px-5 pb-4 sm:px-5">
        <div class="my-3 flex flex-col">
            <h2 class="font-medium tracking-wide text-slate-700 dark:text-navy-100 lg:text-base pb-2">
                {activeGame === "hytale" ? "Hytale Account" : "Xbox Account"}
            </h2>
            <p class="text-sm leading-6 hidden md:block">
                {activeGame === "hytale"
                    ? "Your Hytale identity used for voice chat authentication."
                    : "Your Xbox Live identity used for voice chat authentication."}
            </p>
        </div>

        {#if isReady}
        <div class="flex items-center space-x-4 mt-2 py-3 px-3 rounded-lg">
            {#if gamerpic}
            <img src={gamerpic} alt="Gamerpic" class="size-12 rounded-full" />
            {:else}
            <div class="size-12 rounded-full bg-slate-200 dark:bg-navy-500 flex items-center justify-center">
                <svg xmlns="http://www.w3.org/2000/svg" class="size-6 text-slate-400" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M16 7a4 4 0 11-8 0 4 4 0 018 0zM12 14a7 7 0 00-7 7h14a7 7 0 00-7-7z"/>
                </svg>
            </div>
            {/if}
            <div>
                <span class="text-sm font-medium text-slate-700 dark:text-navy-100">{gamertag || "Unknown"}</span>
                <p class="text-xs text-slate-500 dark:text-navy-300 mt-0.5">{activeGame === "hytale" ? "Hytale Account" : "Xbox Gamertag"}</p>
            </div>
        </div>
        {/if}
    </div>

    {#if activeGame === "minecraft"}
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

        {#if isReady}
        <div class="space-y-3 mt-2">
            {#if minecraftUsername}
            <div class="flex items-center justify-between py-2 px-3 rounded-lg bg-slate-50 dark:bg-navy-600">
                <div>
                    <span class="text-sm font-medium text-slate-700 dark:text-navy-100">{minecraftUsername}</span>
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

            {#if isDesktop}
            <button
                class="btn bg-primary font-medium text-white hover:bg-primary-focus dark:bg-accent dark:hover:bg-accent-focus"
                onclick={handleLinkJavaIdentity}
                disabled={isLinking}
            >
                {#if isLinking}
                    <svg class="animate-spin -ml-1 mr-2 h-4 w-4 text-white" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24">
                        <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
                        <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4z"></path>
                    </svg>
                    Linking...
                {:else}
                    {minecraftUsername ? "Re-link" : "Link Java Identity"}
                {/if}
            </button>
            {:else}
            <p class="text-xs text-slate-500 dark:text-navy-300">
                Java identity linking is available on the desktop app.
            </p>
            {/if}

            {#if linkError}
                <p class="text-xs text-error mt-1">{linkError}</p>
            {/if}
        </div>
        {/if}
    </div>
    {/if}
</div>
