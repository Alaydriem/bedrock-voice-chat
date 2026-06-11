<script lang="ts">
    import type { Snippet } from "svelte";
    import type { BedrockManager } from "../../../js/app/managers/bedrock/BedrockManager";

    interface Props {
        bedrockManager: BedrockManager;
        title: string;
        extraActions?: Snippet;
    }

    let { bedrockManager, title, extraActions }: Props = $props();

    const proxyRunning = bedrockManager.proxyRunning;
    const realmsRunning = bedrockManager.realmsRunning;
    const isLoadingRealms = bedrockManager.isLoadingRealms;
</script>

<div class="flex items-center justify-between">
    <h2 class="font-medium tracking-wide text-slate-700 dark:text-navy-100 lg:text-base">
        {title}
    </h2>
    <div class="flex items-center gap-2">
        {@render extraActions?.()}
        <button
            class="btn bg-slate-150 font-medium text-slate-800 hover:bg-slate-200
                   dark:bg-navy-500 dark:text-navy-100 dark:hover:bg-navy-450
                   disabled:opacity-50 disabled:cursor-not-allowed"
            onclick={() => bedrockManager.refreshRealms()}
            disabled={$isLoadingRealms || $proxyRunning || $realmsRunning}
        >
            {$isLoadingRealms ? "Loading..." : "Refresh"}
        </button>
        <button
            class="btn text-xs+ text-slate-500 hover:text-error dark:text-navy-300 dark:hover:text-error
                   disabled:opacity-50 disabled:cursor-not-allowed disabled:hover:text-slate-500
                   dark:disabled:hover:text-navy-300"
            onclick={() => bedrockManager.signOut()}
            disabled={$proxyRunning || $realmsRunning}
        >
            Sign Out
        </button>
    </div>
</div>
