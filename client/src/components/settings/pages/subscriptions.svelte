<script lang="ts">
    import { onMount } from "svelte";
    import { platform } from "@tauri-apps/plugin-os";
    import { info, error as logError } from "@tauri-apps/plugin-log";
    import type { BedrockManager } from "../../../js/app/managers/bedrock/BedrockManager";

    interface Props {
        bedrockManager: BedrockManager;
    }

    let { bedrockManager }: Props = $props();

    const offers = bedrockManager.offers;

    let busyId = $state<string | null>(null);
    let restoring = $state(false);
    let currentPlatform = $state<string>("");

    onMount(async () => {
        await bedrockManager.loadOffers();
        await bedrockManager.refreshGate();
        try {
            currentPlatform = String(platform()).toLowerCase();
        } catch (e) {
            currentPlatform = "";
            logError(`Platform detection failed: ${e}`);
        }
    });

    async function subscribe(productId: string) {
        busyId = productId;
        try {
            await bedrockManager.purchase(productId);
            info(`Purchase flow completed for ${productId}`);
        } catch (e) {
            logError(`Purchase failed: ${e}`);
        }
        busyId = null;
    }

    async function restore() {
        restoring = true;
        try {
            await bedrockManager.restorePurchases();
        } catch (e) {
            logError(`Restore failed: ${e}`);
        }
        restoring = false;
    }
</script>

<div class="grid grid-cols-1 gap-4 sm:gap-5 lg:gap-6 pt-4 md:pt-0">
    <div class="card p-4 lg:p-6">
        <h2 class="text-lg font-medium text-slate-700 dark:text-navy-100">Realms Connect Subscription</h2>
        <p class="mt-1 text-sm text-slate-500 dark:text-navy-300">
            Subscribing keeps the app-store listing alive and funds upkeep of Realms Connect.
            Proxy Connect stays free.
        </p>
    </div>

    {#if $offers.length > 0}
        <div class="grid grid-cols-1 gap-4 sm:grid-cols-2">
            {#each $offers as offer (offer.product_id)}
                <div class="card p-4 lg:p-6">
                    <h3 class="font-medium text-slate-700 dark:text-navy-100">{offer.title}</h3>
                    <p class="mt-1 text-sm text-slate-500 dark:text-navy-300">{offer.description}</p>
                    <p class="mt-2 text-xl font-semibold text-slate-800 dark:text-navy-50">
                        {offer.formatted_price ?? ""}
                    </p>
                    <button
                        class="btn mt-3 bg-primary font-medium text-white hover:bg-primary-focus disabled:opacity-60"
                        disabled={busyId !== null}
                        onclick={() => subscribe(offer.product_id)}
                    >
                        {busyId === offer.product_id ? "Processing…" : "Subscribe"}
                    </button>
                </div>
            {/each}
        </div>
    {:else}
        <div class="card p-4 lg:p-6">
            <p class="text-sm text-slate-500 dark:text-navy-300">
                No subscription offers are available on this platform right now.
            </p>
        </div>
    {/if}

    <div class="card p-4 lg:p-6">
        <button class="btn border border-slate-300 font-medium dark:border-navy-450 disabled:opacity-60" disabled={restoring} onclick={restore}>
            {restoring ? "Restoring…" : "Restore Purchases"}
        </button>
    </div>

    <div class="card p-4 lg:p-6 border-l-4 border-info">
        <h3 class="font-medium text-slate-700 dark:text-navy-100">Have a store code?</h3>
        <p class="mt-1 text-sm text-slate-500 dark:text-navy-300">
            {#if currentPlatform === "macos" || currentPlatform === "ios"}
                Redeem an App Store code in the App Store app: tap your profile → "Redeem Gift Card or Code."
            {:else if currentPlatform === "android"}
                Redeem a Google Play code in the Play Store app: Profile → Payments &amp; subscriptions → Redeem code.
            {:else if currentPlatform === "windows"}
                Redeem a Microsoft Store code in the Store app: "⋯" menu → Redeem a code.
            {:else}
                Open your platform's store app to redeem a subscription code.
            {/if}
        </p>
    </div>
</div>
