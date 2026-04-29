<script lang="ts">
    import type { ProxyServerEntry } from "../../../js/app/managers/bedrock/ProxyServerEntry";

    interface Props {
        entry: ProxyServerEntry;
        isFavorite: boolean;
        isActive: boolean;
        disabled: boolean;
        onConnect: () => void;
        onDisconnect: () => void;
        onToggleFavorite: () => void;
        onEdit: () => void;
        onDelete: () => void;
    }

    let { entry, isFavorite, isActive, disabled, onConnect, onDisconnect, onToggleFavorite, onEdit, onDelete }: Props = $props();

    let menuOpen = $state(false);

    let hue = $derived.by(() => {
        let total = 0;
        for (let i = 0; i < entry.id.length; i++) {
            total = (total + entry.id.charCodeAt(i)) % 360;
        }
        return total;
    });

    let gradientStyle = $derived(
        `background: linear-gradient(135deg, hsl(${hue}, 55%, 45%), hsl(${(hue + 120) % 360}, 45%, 35%))`
    );
</script>

<div class="card relative overflow-hidden rounded-2xl {isActive ? 'ring-2 ring-success' : ''}">
    <div class="relative h-36" style={gradientStyle}>
        <div class="absolute inset-0 bg-gradient-to-t from-[rgba(0,0,0,0.75)] via-transparent"></div>

        <button
            class="absolute right-3 top-3 flex size-8 items-center justify-center rounded-full
                   bg-black/20 text-white backdrop-blur-sm hover:bg-black/40 transition-colors"
            onclick={(e) => { e.stopPropagation(); onToggleFavorite(); }}
            aria-label={isFavorite ? "Remove from favorites" : "Add to favorites"}
        >
            <svg xmlns="http://www.w3.org/2000/svg" class="size-5 {isFavorite ? 'fill-red-400 text-red-400' : 'fill-none text-white'}" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
                <path stroke-linecap="round" stroke-linejoin="round" d="M4.318 6.318a4.5 4.5 0 000 6.364L12 20.364l7.682-7.682a4.5 4.5 0 00-6.364-6.364L12 7.636l-1.318-1.318a4.5 4.5 0 00-6.364 0z"/>
            </svg>
        </button>

        <span class="absolute left-3 top-3 badge rounded-full bg-primary/80 text-white text-tiny+ backdrop-blur-sm px-2 py-0.5">
            Proxy
        </span>

        <div class="absolute bottom-0 w-full p-4">
            <h3 class="text-lg font-medium text-white line-clamp-1">{entry.name}</h3>
            <p class="mt-1 text-xs text-slate-200 line-clamp-1">{entry.host}:{entry.port}</p>
        </div>
    </div>

    <div class="flex items-center justify-between p-3 bg-white dark:bg-navy-700">
        <div class="relative">
            <button
                class="btn size-8 rounded-full p-0 text-slate-500 hover:bg-slate-100 hover:text-slate-700
                       dark:text-navy-300 dark:hover:bg-navy-600 dark:hover:text-navy-100"
                onclick={(e) => { e.stopPropagation(); menuOpen = !menuOpen; }}
                aria-label="Server actions"
                disabled={isActive}
            >
                <svg xmlns="http://www.w3.org/2000/svg" class="size-5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 5v.01M12 12v.01M12 19v.01M12 6a1 1 0 110-2 1 1 0 010 2zm0 7a1 1 0 110-2 1 1 0 010 2zm0 7a1 1 0 110-2 1 1 0 010 2z"/>
                </svg>
            </button>

            {#if menuOpen}
                <!-- svelte-ignore a11y_no_static_element_interactions -->
                <!-- svelte-ignore a11y_click_events_have_key_events -->
                <div
                    class="fixed inset-0 z-40"
                    onclick={() => { menuOpen = false; }}
                ></div>
                <div class="absolute left-0 bottom-full mb-2 z-50 min-w-[8rem] rounded-lg border border-slate-200 bg-white shadow-lg
                            dark:border-navy-500 dark:bg-navy-600">
                    <button
                        class="block w-full px-4 py-2 text-left text-sm text-slate-700 hover:bg-slate-100
                               dark:text-navy-100 dark:hover:bg-navy-500"
                        onclick={() => { menuOpen = false; onEdit(); }}
                    >
                        Edit
                    </button>
                    <button
                        class="block w-full px-4 py-2 text-left text-sm text-error hover:bg-error/10"
                        onclick={() => { menuOpen = false; onDelete(); }}
                    >
                        Delete
                    </button>
                </div>
            {/if}
        </div>

        {#if isActive}
            <div class="flex items-center gap-2">
                <button
                    class="btn btn-sm font-medium text-white bg-success hover:bg-success-focus"
                    disabled
                >
                    Connected
                </button>
                <button
                    class="btn btn-sm font-medium text-white bg-error hover:bg-error-focus"
                    onclick={onDisconnect}
                >
                    Disconnect
                </button>
            </div>
        {:else}
            <button
                class="btn btn-sm font-medium text-white bg-primary hover:bg-primary-focus dark:bg-accent dark:hover:bg-accent-focus
                       disabled:opacity-50 disabled:cursor-not-allowed disabled:hover:bg-primary dark:disabled:hover:bg-accent"
                onclick={onConnect}
                disabled={disabled}
                title={disabled ? "Disconnect the active session first" : undefined}
            >
                Connect
            </button>
        {/if}
    </div>
</div>
