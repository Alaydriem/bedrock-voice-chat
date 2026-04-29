<script lang="ts">
    import { onMount } from 'svelte';
    import { invoke } from "@tauri-apps/api/core";
    import { error, info } from '@tauri-apps/plugin-log';
    import { Store } from '@tauri-apps/plugin-store';
    import ImageCache from "../js/app/components/imageCache";
    import ImageCacheOptions from "../js/app/components/imageCacheOptions";
    import ConfirmModal from "./ConfirmModal.svelte";
    import { type LoginResponse } from "../js/bindings/LoginResponse";
    import { type ApiConfig } from "../js/bindings/ApiConfig";

    interface ConfigResponse {
        config: ApiConfig;
        client_version: string;
        compatible: boolean;
        client_too_old: boolean;
    }

    type Status = 'checking' | 'connect' | 'reauth' | 'version_mismatch';

    interface Props {
        id: string;
        server: string;
        onRemoved?: () => void;
    }

    let { id, server, onRemoved }: Props = $props();

    let status: Status = $state('checking');
    let buttonMessage = $state("Checking…");
    let badgeMessage = $state("Checking…");
    let badgeClass = $state("bg-slate-500/80");
    let confirmOpen = $state(false);
    let clientTooOld = $state(false);
    let canvasImage = $state("");
    let avatarImage = $state("");

    let hue = $derived.by(() => {
        let total = 0;
        for (let i = 0; i < id.length; i++) {
            total = (total + id.charCodeAt(i)) % 360;
        }
        return total;
    });

    let gradientStyle = $derived(
        `background: linear-gradient(135deg, hsl(${hue}, 55%, 45%), hsl(${(hue + 120) % 360}, 45%, 35%))`
    );

    let displayHost = $derived(server.replace(/^https?:\/\//, ""));

    const imageCacheTtl = 60 * 60 * 24 * 7;
    const imageCacher = new ImageCache();

    onMount(async () => {
        loadImages();
        await checkServer();
    });

    function loadImages() {
        imageCacher
            .getImage(new ImageCacheOptions(`${server}/assets/canvas.png`, imageCacheTtl))
            .then((url) => { canvasImage = url; })
            .catch(() => { canvasImage = ""; });

        imageCacher
            .getImage(new ImageCacheOptions(`${server}/assets/avatar.png`, imageCacheTtl))
            .then((url) => { avatarImage = url; })
            .catch(() => { avatarImage = ""; });
    }

    async function checkServer() {
        status = 'checking';
        buttonMessage = "Checking…";
        badgeMessage = "Checking…";
        badgeClass = "bg-slate-500/80";

        try {
            const credentials = await invoke<LoginResponse>("get_credentials", { server });

            const expired = await invoke<boolean>("is_certificate_expired", { server });
            if (expired) {
                showReauth();
                return;
            }

            await invoke("api_initialize_client", {
                endpoint: server,
                cert: credentials.certificate_ca,
                pem: credentials.certificate + credentials.certificate_key
            });

            try {
                await invoke("refresh_server_state", { server });
            } catch (e) {
                // Non-fatal: continue with cached credentials
            }

            const configResponse = await invoke<ConfigResponse>("api_get_config", { server });

            if (!configResponse.compatible) {
                showVersionMismatch(configResponse.client_too_old, configResponse.config.protocol_version, configResponse.client_version);
                return;
            }

            showConnect();
        } catch (e) {
            error(`Failed to check server ${server}: ${e}`);
            showReauth();
        }
    }

    function showConnect() {
        status = 'connect';
        buttonMessage = "Connect";
        badgeMessage = "Online";
        badgeClass = "bg-success/80";
    }

    function showReauth() {
        status = 'reauth';
        buttonMessage = "Re-authenticate";
        badgeMessage = "Auth required";
        badgeClass = "bg-error/80";
    }

    function showVersionMismatch(tooOld: boolean, serverVersion: string, clientVersion: string) {
        status = 'version_mismatch';
        clientTooOld = tooOld;
        buttonMessage = tooOld ? `Update Client (${clientVersion} → ${serverVersion})` : "Server Outdated";
        badgeMessage = "Outdated";
        badgeClass = "bg-warning/80";
    }

    async function handleAction() {
        if (status === 'version_mismatch' || status === 'checking') return;

        if (status === 'connect') {
            const store = await Store.load("store.json", { autoSave: false, defaults: {} });
            await store.set("current_server", server);
            const serverList = await store.get("server_list") as Array<{ server: string, player: string, game?: string }> | null;
            const entry = serverList?.find(s => s.server === server);
            if (entry) {
                await store.set("current_player", entry.player);
                await store.set("active_game", entry.game || "minecraft");
            }
            await store.save();
            window.location.href = `/dashboard?server=${server}`;
        } else {
            window.location.href = `/login?reauth=true&server=${server}`;
        }
    }

    async function handleRemoveConfirmed() {
        try {
            await invoke("delete_credentials", { server }).catch((e) => {
                info(`delete_credentials failed for ${server}: ${e}`);
            });

            const store = await Store.load("store.json", { autoSave: false, defaults: {} });
            const serverList = (await store.get("server_list") as Array<{ server: string, player: string, game?: string }> | null) ?? [];
            const filtered = serverList.filter(s => s.server !== server);
            await store.set("server_list", filtered);

            const currentServer = await store.get("current_server") as string | null;
            if (currentServer === server) {
                await store.delete("current_server");
                await store.delete("current_player");
            }
            await store.save();

            confirmOpen = false;

            if (filtered.length === 0) {
                window.location.href = "/login";
                return;
            }

            onRemoved?.();
        } catch (e) {
            error(`Failed to remove server ${server}: ${e}`);
            confirmOpen = false;
        }
    }

    let buttonClasses = $derived.by(() => {
        switch (status) {
            case 'connect':
                return "bg-success hover:bg-success-focus text-white";
            case 'reauth':
                return "bg-error hover:bg-error-focus text-white";
            case 'version_mismatch':
                return "bg-warning text-slate-800 cursor-not-allowed";
            case 'checking':
            default:
                return "bg-slate-200 text-slate-500 dark:bg-navy-600 dark:text-navy-300 cursor-wait";
        }
    });
</script>

<div class="card relative overflow-hidden rounded-2xl">
    <div class="relative h-56" style={gradientStyle}>
        {#if canvasImage}
            <img
                src={canvasImage}
                alt=""
                class="absolute inset-0 h-full w-full object-cover"
            />
        {/if}
        <div class="absolute inset-0 bg-gradient-to-t from-[rgba(0,0,0,0.85)] via-[rgba(0,0,0,0.15)] to-transparent"></div>

        <span class="absolute left-4 top-4 badge rounded-full {badgeClass} text-white text-tiny+ backdrop-blur-sm px-2.5 py-1 font-medium">
            {badgeMessage}
        </span>

        {#if avatarImage}
            <div class="mask is-hexagon absolute right-4 top-4 size-20 bg-black/30 backdrop-blur-sm">
                <img
                    src={avatarImage}
                    alt="Server avatar"
                    class="h-full w-full object-cover"
                />
            </div>
        {/if}

        <div class="absolute bottom-0 w-full p-5">
            <h3 class="text-xl font-semibold text-white line-clamp-1" title={server}>{displayHost}</h3>
            <p class="mt-1 text-sm text-slate-200 line-clamp-1">{server}</p>
        </div>
    </div>

    <div class="flex items-center justify-between p-4 bg-white dark:bg-navy-700">
        <div class="flex items-center gap-2">
            <button
                class="btn size-10 rounded-full p-0 text-error hover:bg-error/10
                       disabled:opacity-50 disabled:cursor-not-allowed"
                onclick={() => { confirmOpen = true; }}
                disabled={status === 'checking'}
                aria-label="Remove server"
                title="Remove server"
            >
                <svg xmlns="http://www.w3.org/2000/svg" class="size-5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6M1 7h22M9 7V4a1 1 0 011-1h4a1 1 0 011 1v3"/>
                </svg>
            </button>

            <button
                class="btn size-10 rounded-full p-0 text-slate-500 hover:bg-slate-100 hover:text-slate-700
                       dark:text-navy-300 dark:hover:bg-navy-600 dark:hover:text-navy-100
                       disabled:opacity-50 disabled:cursor-not-allowed"
                onclick={checkServer}
                disabled={status === 'checking'}
                aria-label="Refresh server status"
                title="Refresh"
            >
                <svg xmlns="http://www.w3.org/2000/svg" class="size-5 {status === 'checking' ? 'animate-spin' : ''}" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15"/>
                </svg>
            </button>
        </div>

        <button
            class="btn font-medium {buttonClasses}"
            onclick={handleAction}
            disabled={status === 'checking' || status === 'version_mismatch'}
            title={status === 'version_mismatch' && !clientTooOld ? "Server is running an older protocol" : undefined}
        >
            {buttonMessage}
        </button>
    </div>
</div>

<ConfirmModal
    open={confirmOpen}
    title="Remove server?"
    message={`This will remove ${displayHost} from your server list and clear its saved credentials. You'll need to sign in again to reconnect.`}
    confirmLabel="Remove"
    cancelLabel="Cancel"
    confirmVariant="danger"
    onConfirm={handleRemoveConfirmed}
    onCancel={() => { confirmOpen = false; }}
/>
