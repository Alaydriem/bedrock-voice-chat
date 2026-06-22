<script lang="ts">
    import type { BedrockManager } from "../../../js/app/managers/bedrock/BedrockManager";
    import { RealmsUpsellManager } from "../../../js/app/managers/bedrock/RealmsUpsellManager";
    import UpsellHero from "./upsell/UpsellHero.svelte";
    import PlanCard from "./upsell/PlanCard.svelte";
    import PlanSkeleton from "./upsell/PlanSkeleton.svelte";
    import SameServerNotice from "./upsell/SameServerNotice.svelte";
    import HowItWorks from "./upsell/HowItWorks.svelte";
    import RestoreCard from "./upsell/RestoreCard.svelte";

    interface Props {
        bedrockManager: BedrockManager;
    }

    let { bedrockManager }: Props = $props();

    const manager = new RealmsUpsellManager(bedrockManager);

    const offersLoaded = manager.offersLoaded;
    const sortedOffers = manager.sortedOffers;

    const offersGridClass = $derived(
        $offersLoaded && $sortedOffers.length === 1
            ? "mx-auto grid max-w-xs grid-cols-1 gap-4 sm:gap-5 lg:gap-6"
            : "mx-auto grid max-w-2xl grid-cols-1 gap-4 sm:grid-cols-2 sm:gap-5 lg:gap-6",
    );
</script>

<div class="mx-auto w-full max-w-4xl pt-4 md:pt-0">
    <UpsellHero />

    <section class={offersGridClass}>
        {#if !$offersLoaded}
            <PlanSkeleton featured />
            <PlanSkeleton />
        {:else if $sortedOffers.length > 0}
            {#each $sortedOffers as offer (offer.product_id)}
                <PlanCard {offer} {manager} />
            {/each}
        {:else}
            <div class="card p-5 text-center sm:col-span-2">
                <p class="text-sm text-slate-500 dark:text-navy-300">
                    No subscription offers are available on this platform right now. If you've
                    already subscribed, tap <span class="font-medium">Restore purchases</span> below.
                </p>
            </div>
        {/if}
    </section>

    <SameServerNotice />

    <HowItWorks />

    <RestoreCard {manager} />

    <p class="mt-6 text-center text-xs text-slate-400 dark:text-navy-300">
        Billing is handled securely by your platform's store. Subscriptions renew automatically and
        can be cancelled anytime from your store account.
    </p>
</div>
