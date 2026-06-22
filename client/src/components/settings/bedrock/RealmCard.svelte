<script lang="ts">
    import type { RealmEntry } from "../../../js/bindings/RealmEntry";

    interface Props {
        realm: RealmEntry;
        isFavorite: boolean;
        isActive: boolean;
        disabled: boolean;
        onConnect: () => void;
        onDisconnect: () => void;
        onToggleFavorite: () => void;
    }

    let { realm, isFavorite, isActive, disabled, onConnect, onDisconnect, onToggleFavorite }: Props = $props();

    let gradientStyle = $derived(
        `background: linear-gradient(135deg, hsl(${(Number(realm.id) * 47) % 360}, 55%, 45%), hsl(${(Number(realm.id) * 47 + 120) % 360}, 45%, 35%))`
    );
</script>

<div class="card relative overflow-hidden rounded-2xl {isActive ? 'ring-2 ring-success' : ''}">
    <div class="relative h-36" style={gradientStyle}>
        <div class="absolute inset-0 bg-gradient-to-t from-[rgba(0,0,0,0.75)] via-transparent"></div>

        <!-- Favorite button -->
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

        <!-- State badge -->
        {#if realm.state === "OPEN"}
            <span class="absolute left-3 top-3 badge rounded-full bg-success/80 text-white text-tiny+ backdrop-blur-sm px-2 py-0.5">
                Online
            </span>
        {:else}
            <span class="absolute left-3 top-3 badge rounded-full bg-slate-500/80 text-white text-tiny+ backdrop-blur-sm px-2 py-0.5">
                {realm.state}
            </span>
        {/if}

        <!-- Card body overlaid at bottom -->
        <div class="absolute bottom-0 w-full p-4">
            <h3 class="text-lg font-medium text-white line-clamp-1">{realm.name}</h3>
            <p class="mt-1 text-xs text-slate-200 line-clamp-1">ID: {realm.id}</p>
        </div>
    </div>

    <!-- Action area -->
    <div class="flex items-center justify-end p-3 bg-white dark:bg-navy-700">
        {#if isActive}
            <div class="flex items-center gap-2">
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
