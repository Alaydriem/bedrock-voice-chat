<script lang="ts">
    import type { BedrockManager } from "../../../js/app/managers/bedrock/BedrockManager";

    interface Props {
        bedrockManager: BedrockManager;
    }

    let { bedrockManager }: Props = $props();
    const gatingModal = bedrockManager.gatingModal;

    function close() {
        bedrockManager.dismissGatingModal();
    }

    function goToSubscriptions() {
        close();
        window.dispatchEvent(new CustomEvent("settings-navigate", { detail: "subscriptions.svelte" }));
    }
</script>

{#if $gatingModal}
    <div class="fixed inset-0 z-50 flex items-center justify-center bg-slate-900/60 p-4">
        <div class="card w-full max-w-md p-6">
            <h3 class="text-lg font-medium text-slate-700 dark:text-navy-100">Realms Connect is locked</h3>
            <p class="mt-2 text-sm text-slate-500 dark:text-navy-300">
                {#if $gatingModal.status === "not_entitled"}
                    You need an active subscription or a membership code to connect to Realms.
                {:else if $gatingModal.status === "feature_disabled"}
                    Realms Connect is currently unavailable. Please check back later.
                {:else}
                    Realms Connect could not be started.
                {/if}
            </p>
            <div class="mt-4 flex justify-end gap-2">
                <button class="btn border border-slate-300 font-medium dark:border-navy-450" onclick={close}>Close</button>
                {#if $gatingModal.status === "not_entitled"}
                    <button class="btn bg-primary font-medium text-white hover:bg-primary-focus" onclick={goToSubscriptions}>
                        View options
                    </button>
                {/if}
            </div>
        </div>
    </div>
{/if}
