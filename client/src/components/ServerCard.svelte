<script lang="ts">
    import { onMount } from 'svelte';
    import ImageCache from "../js/app/components/imageCache";
    import ConfirmModal from "./ConfirmModal.svelte";
    import { ServerCardManager } from "../js/app/managers/server_card/ServerCardManager";
    import { ServerHealthService } from "../js/app/services/ServerHealthService";
    import { ServerListStore } from "../js/app/services/ServerListStore";
    import type { NextAction } from "../js/app/managers/server_card/NextAction";

    interface Props {
        id: string;
        server: string;
        onRemoved?: () => void;
    }

    let { id, server, onRemoved }: Props = $props();

    let confirmOpen = $state(false);

    const manager = new ServerCardManager(
        { id, server },
        {
            health: new ServerHealthService(),
            serverList: new ServerListStore(),
            imageCache: new ImageCache(),
        },
    );

    const status = manager.status;
    const button = manager.button;
    const badge = manager.badge;
    const canvasImage = manager.canvasImage;
    const avatarImage = manager.avatarImage;

    onMount(async () => {
        await manager.initialize();
    });

    function applyAction(action: NextAction) {
        if (action.kind === 'navigate') {
            window.location.href = action.href;
        }
    }

    async function handleRefresh() {
        await manager.refresh();
    }

    async function handleAction() {
        applyAction(await manager.handleAction());
    }

    async function handleRemoveConfirmed() {
        const action = await manager.remove();
        confirmOpen = false;
        if (action.kind === 'navigate') {
            applyAction(action);
        } else {
            onRemoved?.();
        }
    }
</script>

<div class="card relative overflow-hidden rounded-2xl">
    <div class="relative h-56" style={manager.gradientStyle}>
        {#if $canvasImage}
            <img
                src={$canvasImage}
                alt=""
                class="absolute inset-0 h-full w-full object-cover"
            />
        {/if}
        <div class="absolute inset-0 bg-gradient-to-t from-[rgba(0,0,0,0.85)] via-[rgba(0,0,0,0.15)] to-transparent"></div>

        <span class="absolute left-4 top-4 badge rounded-full {$badge.classes} text-white text-tiny+ backdrop-blur-sm px-2.5 py-1 font-medium">
            {$badge.label}
        </span>

        {#if $avatarImage}
            <div class="mask is-hexagon absolute right-4 top-4 size-20 bg-black/30 backdrop-blur-sm">
                <img
                    src={$avatarImage}
                    alt="Server avatar"
                    class="h-full w-full object-cover"
                />
            </div>
        {/if}

        <div class="absolute bottom-0 w-full p-5">
            <h3 class="text-xl font-semibold text-white line-clamp-1" title={server}>{manager.displayHost}</h3>
            <p class="mt-1 text-sm text-slate-200 line-clamp-1">{server}</p>
        </div>
    </div>

    <div class="flex items-center justify-between p-4 bg-white dark:bg-navy-700">
        <div class="flex items-center gap-2">
            <button
                class="btn size-10 rounded-full p-0 text-error hover:bg-error/10
                       disabled:opacity-50 disabled:cursor-not-allowed"
                onclick={() => { confirmOpen = true; }}
                disabled={$status === 'checking'}
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
                onclick={handleRefresh}
                disabled={$status === 'checking'}
                aria-label="Refresh server status"
                title="Refresh"
            >
                <svg xmlns="http://www.w3.org/2000/svg" class="size-5 {$status === 'checking' ? 'animate-spin' : ''}" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15"/>
                </svg>
            </button>
        </div>

        <button
            class="btn font-medium {$button.classes}"
            onclick={handleAction}
            disabled={$button.disabled}
            title={$button.title}
        >
            {$button.label}
        </button>
    </div>
</div>

<ConfirmModal
    open={confirmOpen}
    title="Remove server?"
    message={`This will remove ${manager.displayHost} from your server list and clear its saved credentials. You'll need to sign in again to reconnect.`}
    confirmLabel="Remove"
    cancelLabel="Cancel"
    confirmVariant="danger"
    onConfirm={handleRemoveConfirmed}
    onCancel={() => { confirmOpen = false; }}
/>
