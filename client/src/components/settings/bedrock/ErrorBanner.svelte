<script lang="ts">
    import type { BedrockManager } from "../../../js/app/managers/bedrock/BedrockManager";

    interface Props {
        bedrockManager: BedrockManager;
    }

    let { bedrockManager }: Props = $props();

    const connectionError = bedrockManager.connectionError;
</script>

{#if $connectionError}
    <div class="card p-4 border-l-4 {$connectionError.severity === 'warning' ? 'border-warning' : 'border-error'}">
        <div class="flex items-start gap-3">
            <svg
                xmlns="http://www.w3.org/2000/svg"
                class="size-5 mt-0.5 shrink-0 {$connectionError.severity === 'warning' ? 'text-warning' : 'text-error'}"
                fill="none"
                viewBox="0 0 24 24"
                stroke="currentColor"
            >
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-2.5L13.732 4c-.77-.833-1.964-.833-2.732 0L4.082 16.5c-.77.833.192 2.5 1.732 2.5z"/>
            </svg>
            <div class="flex-1 min-w-0">
                <p class="text-sm font-medium text-slate-800 dark:text-navy-100">
                    {$connectionError.title}
                </p>
                <p class="mt-1 text-sm text-slate-600 dark:text-navy-200">{$connectionError.detail}</p>
                <p class="mt-1 text-sm text-slate-500 dark:text-navy-300">{$connectionError.suggestion}</p>
                <p class="mt-1 text-tiny+ text-slate-400 dark:text-navy-300 font-mono break-all">
                    {$connectionError.raw.kind}
                </p>
                <div class="flex flex-wrap items-center gap-2 mt-3">
                    <button
                        class="btn btn-sm bg-primary font-medium text-white hover:bg-primary-focus dark:bg-accent dark:hover:bg-accent-focus"
                        onclick={() => bedrockManager.refreshRealms()}
                    >
                        Refresh tokens
                    </button>
                    <button
                        class="btn btn-sm text-slate-500 hover:text-slate-700 dark:text-navy-300 dark:hover:text-navy-100"
                        onclick={() => bedrockManager.dismissConnectionError()}
                    >
                        Dismiss
                    </button>
                </div>
            </div>
        </div>
    </div>
{/if}
