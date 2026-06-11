<script lang="ts">
    import type { BedrockManager } from "../../../js/app/managers/bedrock/BedrockManager";

    interface Props {
        bedrockManager: BedrockManager;
    }

    let { bedrockManager }: Props = $props();

    const deviceCode = bedrockManager.deviceCode;
    const deviceUrl = bedrockManager.deviceUrl;
    const loginError = bedrockManager.loginError;
    const codeCopied = bedrockManager.codeCopied;
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<!-- svelte-ignore a11y_click_events_have_key_events -->
<div
    class="fixed inset-0 z-50 flex items-center justify-center bg-black/50 backdrop-blur-sm"
    onclick={(e) => { if (e.target === e.currentTarget) bedrockManager.closeLoginModal(); }}
>
    <div class="card w-full max-w-md rounded-2xl p-6 shadow-xl bg-white dark:bg-navy-700">
        <div class="flex items-center justify-between mb-4">
            <h3 class="text-lg font-medium text-slate-800 dark:text-navy-100">
                Xbox Live Sign In
            </h3>
            <button
                class="btn size-8 rounded-full p-0 hover:bg-slate-300/20 dark:hover:bg-navy-300/20"
                onclick={() => bedrockManager.closeLoginModal()}
                aria-label="Close"
            >
                <svg xmlns="http://www.w3.org/2000/svg" class="size-5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"/>
                </svg>
            </button>
        </div>

        {#if $loginError}
            <div class="rounded-lg bg-error/10 p-4">
                <p class="text-sm text-error">{$loginError}</p>
            </div>
            <div class="mt-4 flex justify-end">
                <button
                    class="btn bg-slate-150 font-medium text-slate-800 hover:bg-slate-200 dark:bg-navy-500 dark:text-navy-100 dark:hover:bg-navy-450"
                    onclick={() => bedrockManager.closeLoginModal()}
                >
                    Close
                </button>
            </div>
        {:else if $deviceCode}
            <p class="text-sm text-slate-600 dark:text-navy-200">
                Enter this code at the Microsoft sign-in page:
            </p>

            <div class="mt-4 rounded-lg bg-slate-50 dark:bg-navy-600 p-5 text-center">
                <p class="font-mono text-3xl font-bold tracking-[0.3em] text-primary dark:text-accent-light select-all">
                    {$deviceCode}
                </p>
            </div>

            <div class="mt-4 flex gap-3">
                <button
                    class="btn flex-1 bg-primary font-medium text-white hover:bg-primary-focus dark:bg-accent dark:hover:bg-accent-focus"
                    onclick={() => bedrockManager.openLoginUrl()}
                >
                    Open Sign-In Page
                </button>
                <button
                    class="btn bg-slate-150 font-medium text-slate-800 hover:bg-slate-200 dark:bg-navy-500 dark:text-navy-100 dark:hover:bg-navy-450"
                    onclick={() => bedrockManager.copyDeviceCode()}
                >
                    {$codeCopied ? "Copied!" : "Copy Code"}
                </button>
            </div>

            <div class="mt-4 flex items-center gap-2">
                <div class="size-4 animate-spin rounded-full border-2 border-slate-300 border-t-primary dark:border-navy-400 dark:border-t-accent"></div>
                <p class="text-xs text-slate-500 dark:text-navy-300">Waiting for you to complete sign-in...</p>
            </div>
        {:else}
            <div class="flex flex-col items-center gap-3 py-8">
                <div class="size-8 animate-spin rounded-full border-2 border-slate-300 border-t-primary dark:border-navy-400 dark:border-t-accent"></div>
                <p class="text-sm text-slate-500 dark:text-navy-300">Contacting Xbox Live...</p>
            </div>
        {/if}
    </div>
</div>
