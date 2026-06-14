<script lang="ts">
    import type { RealmsUpsellManager } from "../../../../js/app/managers/bedrock/RealmsUpsellManager";

    interface Props {
        manager: RealmsUpsellManager;
    }

    let { manager }: Props = $props();

    const busyId = manager.busyId;
    const restoring = manager.restoring;
</script>

<div class="mt-8">
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
            disabled={$restoring || $busyId !== null}
            onclick={() => manager.restore()}
        >
            {$restoring ? "Restoring…" : "Restore purchases"}
        </button>
    </div>
</div>
