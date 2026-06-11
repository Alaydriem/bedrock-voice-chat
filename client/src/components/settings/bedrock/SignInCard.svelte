<script lang="ts">
    import type { BedrockManager } from "../../../js/app/managers/bedrock/BedrockManager";

    interface Props {
        bedrockManager: BedrockManager;
        description: string;
    }

    let { bedrockManager, description }: Props = $props();
</script>

<div class="card p-4 lg:p-6">
    <h2 class="font-medium tracking-wide text-slate-700 dark:text-navy-100 lg:text-base pb-2">
        Xbox Live Authentication
    </h2>
    <p class="text-sm text-slate-500 dark:text-navy-300">
        {description}
    </p>
    <button
        class="btn mt-4 bg-success font-medium text-white hover:bg-success-focus"
        onclick={() => bedrockManager.openLoginModal()}
    >
        Sign in with Xbox Live
    </button>
</div>
