<script lang="ts">
    import { info, error as logError } from "@tauri-apps/plugin-log";
    import type { IapOffer } from "../../../js/bindings/IapOffer";
    import type { BedrockManager } from "../../../js/app/managers/bedrock/BedrockManager";

    interface Props {
        bedrockManager: BedrockManager;
    }

    let { bedrockManager }: Props = $props();

    const offers = bedrockManager.offers;
    const offersLoaded = bedrockManager.offersLoaded;

    const STEPS = [
        {
            title: "Add the BVC add-on to your Realm",
            body: "Install the no-network BVC behavior pack on the Realm you want voice on. It's a one-time setup.",
        },
        {
            title: "Everyone connects to the same BVC server",
            body: "Voice is carried by a BVC server — you and every player you want to hear must be connected to the same one.",
        },
        {
            title: "Join your Realm and talk",
            body: "Hop into the Realm in Minecraft and you'll hear everyone in proximity voice.",
        },
    ];

    let busyId = $state<string | null>(null);
    let restoring = $state(false);

    const sortedOffers = $derived(
        [...$offers].sort((a, b) => Number(isAnnual(b)) - Number(isAnnual(a))),
    );

    const offersGridClass = $derived(
        $offersLoaded && sortedOffers.length === 1
            ? "mx-auto grid max-w-xs grid-cols-1 gap-4 sm:gap-5 lg:gap-6"
            : "mx-auto grid max-w-2xl grid-cols-1 gap-4 sm:grid-cols-2 sm:gap-5 lg:gap-6",
    );

    function isAnnual(offer: IapOffer): boolean {
        return offer.product_id.toLowerCase().includes("annual");
    }

    function cadence(offer: IapOffer): string {
        const id = offer.product_id.toLowerCase();
        if (id.includes("annual") || id.includes("year")) return "/ year";
        if (id.includes("month")) return "/ month";
        return "";
    }

    function gradientStyle(offer: IapOffer): string {
        let hue = 0;
        for (let i = 0; i < offer.product_id.length; i++) {
            hue = (hue + offer.product_id.charCodeAt(i)) % 360;
        }
        return `background: linear-gradient(135deg, hsl(${hue}, 55%, 45%), hsl(${(hue + 120) % 360}, 45%, 35%))`;
    }

    async function subscribe(productId: string) {
        busyId = productId;
        try {
            await bedrockManager.purchase(productId);
            info(`Purchase flow completed for ${productId}`);
        } catch (e) {
            logError(`Purchase failed: ${e}`);
        } finally {
            busyId = null;
        }
    }

    async function restore() {
        restoring = true;
        try {
            await bedrockManager.restorePurchases();
        } catch (e) {
            logError(`Restore failed: ${e}`);
        } finally {
            restoring = false;
        }
    }
</script>

{#snippet planCard(offer: IapOffer)}
    {@const featured = isAnnual(offer)}
    <div
        class="h-full {featured
            ? 'rounded-2xl bg-gradient-to-r from-violet-400 to-purple-600 p-1'
            : ''}"
    >
        <div class="card flex h-full flex-col overflow-hidden rounded-2xl">
        <div
            class="relative overflow-hidden rounded-t-2xl p-5 text-center"
            style={gradientStyle(offer)}
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
                    {#if cadence(offer)}
                        <p class="mt-1 text-sm font-medium text-white/80">
                            {cadence(offer)}
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
                disabled={busyId !== null || restoring}
                onclick={() => subscribe(offer.product_id)}
            >
                {#if busyId === offer.product_id}
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
{/snippet}

{#snippet planSkeleton(featured: boolean)}
    <div class="card h-full animate-pulse overflow-hidden rounded-2xl {featured ? 'ring-2 ring-slate-200 dark:ring-navy-600' : ''}">
        <div class="rounded-t-2xl p-5 text-center {featured ? 'bg-slate-200 dark:bg-navy-700' : 'bg-slate-150 dark:bg-navy-800'}">
            <div class="mx-auto h-4 w-20 rounded bg-slate-300/70 dark:bg-navy-600"></div>
            <div class="mx-auto mt-3 h-9 w-28 rounded bg-slate-300/70 dark:bg-navy-600"></div>
            <div class="mx-auto mt-3 h-3 w-16 rounded bg-slate-300/70 dark:bg-navy-600"></div>
        </div>
        <div class="p-5">
            <div class="mx-auto h-3 w-40 rounded bg-slate-200 dark:bg-navy-600"></div>
            <div class="mt-5 h-9 w-full rounded-full bg-slate-200 dark:bg-navy-600"></div>
        </div>
    </div>
{/snippet}

<div class="mx-auto w-full max-w-4xl pt-4 md:pt-0">
    <section class="py-6 text-center">
        <img
            src="/images/app-logo-transparent.svg"
            alt="Bedrock Voice Chat"
            class="mx-auto h-36 w-auto"
        />
        <h2 class="mt-1 text-2xl font-semibold text-slate-700 dark:text-navy-100 sm:text-2xl">
            Proximity Voice Chat for Minecraft Bedrock Realms
        </h2>
        <p class="mx-auto mt-2 max-w-xl text-sm leading-relaxed text-slate-500 dark:text-navy-300">
            Add the BVC add-on to your Realm and talk in proximity voice when connected to the same BVC server! Realms
            Connect requires an active subscription.
        </p>
        <div class="mt-4 flex flex-wrap items-center justify-center gap-2">
            <span class="badge rounded-full bg-info/12 text-info dark:bg-info/15">Cancel anytime</span>
        </div>
    </section>

    <section class={offersGridClass}>
        {#if !$offersLoaded}
            {@render planSkeleton(true)}
            {@render planSkeleton(false)}
        {:else if sortedOffers.length > 0}
            {#each sortedOffers as offer (offer.product_id)}
                {@render planCard(offer)}
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

    <div class="card mx-auto mt-6 max-w-2xl border-l-4 border-warning p-4 text-left">
        <p class="text-sm font-semibold text-slate-800 dark:text-navy-100">
            Everyone must be on the same BVC server
        </p>
        <p class="mt-1 text-sm leading-relaxed text-slate-600 dark:text-navy-200">
            Voice is carried by a BVC server. Every players must be connected to the same Bedrock Voice Chat server to hear each other.
        </p>
    </div>

    <section class="mt-8">
        <h3 class="text-center font-medium text-slate-700 dark:text-navy-100">How it works</h3>
        <div class="mt-4 grid grid-cols-1 gap-4 sm:grid-cols-3 sm:gap-5 lg:gap-6">
            {#each STEPS as step, i}
                <div class="card p-5">
                    <p class="text-xs+ font-semibold uppercase tracking-wide text-primary dark:text-accent-light">
                        Step {i + 1}
                    </p>
                    <h4 class="mt-1 font-medium text-slate-700 dark:text-navy-100">{step.title}</h4>
                    <p class="mt-1 text-sm leading-relaxed text-slate-500 dark:text-navy-300">
                        {step.body}
                    </p>
                </div>
            {/each}
        </div>
    </section>

    <section class="mt-8">
        <div class="card mx-auto flex max-w-2xl flex-col items-center gap-4 p-5 text-center sm:flex-row sm:justify-between sm:text-left">
            <div>
                <h3 class="font-medium text-slate-700 dark:text-navy-100">Already subscribed?</h3>
                <p class="mt-1 text-sm text-slate-500 dark:text-navy-300">
                    Bought Realms Connect on another device or reinstalled the app? Restore your
                    subscription from your platform store.
                </p>
            </div>
            <button
                class="btn w-full shrink-0 rounded-full border border-slate-300 font-medium text-slate-700 hover:bg-slate-100 focus:bg-slate-100 disabled:pointer-events-none disabled:opacity-60 dark:border-navy-450 dark:text-navy-100 dark:hover:bg-navy-500 dark:focus:bg-navy-500 sm:w-auto sm:px-6"
                disabled={restoring || busyId !== null}
                onclick={restore}
            >
                {restoring ? "Restoring…" : "Restore purchases"}
            </button>
        </div>
    </section>

    <p class="mt-6 text-center text-xs text-slate-400 dark:text-navy-300">
        Billing is handled securely by your platform's store. Subscriptions renew automatically and
        can be cancelled anytime from your store account.
    </p>
</div>
