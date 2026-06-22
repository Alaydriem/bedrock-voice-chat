<script lang="ts">
    import type { IapOffer } from "../../../../js/bindings/IapOffer";
    import type { RealmsUpsellManager } from "../../../../js/app/managers/bedrock/RealmsUpsellManager";

    interface Props {
        offer: IapOffer;
        manager: RealmsUpsellManager;
    }

    let { offer, manager }: Props = $props();

    const busyId = manager.busyId;
    const restoring = manager.restoring;

    let featured = $derived(manager.isAnnual(offer));
</script>

<div
    class="h-full {featured
        ? 'rounded-2xl bg-gradient-to-r from-violet-400 to-purple-600 p-1'
        : ''}"
>
    <div class="card flex h-full flex-col overflow-hidden rounded-2xl">
        <div
            class="relative overflow-hidden rounded-t-2xl p-5 text-center"
            style={manager.gradientStyle(offer)}
        >
            <div class="absolute inset-0 bg-gradient-to-t from-black/60 via-black/25 to-black/20"></div>

            <div class="relative">
                <p class="text-lg font-medium text-white">
                    {offer.title}
                </p>

                {#if offer.formatted_price}
                    <p class="mt-2 text-4xl font-semibold tracking-tight text-white">
                        {offer.formatted_price}
                    </p>
                    {#if manager.cadence(offer)}
                        <p class="mt-1 text-sm font-medium text-white/80">
                            {manager.cadence(offer)}
                        </p>
                    {/if}
                {/if}
            </div>
        </div>

        <div class="flex flex-1 flex-col p-5 text-center">
            <p class="mb-5 text-sm leading-relaxed text-slate-500 dark:text-navy-300">
                {offer.description}
            </p>

            <button
                class="btn mt-auto w-full rounded-full font-medium disabled:pointer-events-none disabled:opacity-60 {featured
                    ? 'bg-primary text-white hover:bg-primary-focus focus:bg-primary-focus active:bg-primary-focus/90 dark:bg-accent dark:hover:bg-accent-focus dark:focus:bg-accent-focus'
                    : 'border border-slate-300 text-primary hover:bg-slate-100 focus:bg-slate-100 dark:border-navy-450 dark:text-accent-light dark:hover:bg-navy-500 dark:focus:bg-navy-500'}"
                disabled={$busyId !== null || $restoring}
                onclick={() => manager.subscribe(offer.product_id)}
            >
                {#if $busyId === offer.product_id}
                    <svg
                        class="size-4 animate-spin"
                        xmlns="http://www.w3.org/2000/svg"
                        fill="none"
                        viewBox="0 0 24 24"
                    >
                        <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
                        <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4z"></path>
                    </svg>
                    <span class="ml-2">Processing…</span>
                {:else}
                    Subscribe
                {/if}
            </button>
        </div>
    </div>
</div>
