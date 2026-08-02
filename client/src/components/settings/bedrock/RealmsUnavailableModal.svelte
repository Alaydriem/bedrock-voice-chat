<script lang="ts">
    import type { BedrockManager } from "../../../js/app/managers/bedrock/BedrockManager";

    interface Props {
        bedrockManager: BedrockManager;
    }

    let { bedrockManager }: Props = $props();
    const showModal = bedrockManager.realmsUnavailableModal;

    function close() {
        bedrockManager.dismissRealmsUnavailableModal();
    }
</script>

{#if $showModal}
    <div class="fixed inset-0 z-50 flex items-center justify-center bg-slate-900/60 p-4">
        <div class="card w-full max-w-md p-6">
            <h3 class="text-lg font-medium text-slate-700 dark:text-navy-100">Realms Connect is unavailable</h3>
            <p class="mt-2 text-sm text-slate-500 dark:text-navy-300">
                Realms Connect is temporarily unavailable. Please check back later.
            </p>
            <div class="mt-4 flex justify-end">
                <button class="btn bg-primary font-medium text-white hover:bg-primary-focus" onclick={close}>
                    Close
                </button>
            </div>
        </div>
    </div>
{/if}
