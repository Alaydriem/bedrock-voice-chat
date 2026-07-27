<script lang="ts">
    import type { BedrockCapabilityStatus } from "../../../js/app/managers/bedrock/BedrockCapabilityManager";

    interface Props {
        status: BedrockCapabilityStatus | null;
        serverHost: string;
        isChecking: boolean;
        onRetry: () => void;
    }
    let { status, serverHost, isChecking, onRetry }: Props = $props();

    // The check usually completes in milliseconds, so a spinner alone would be
    // an invisible flash. When a check finishes and the state is unchanged,
    // confirm briefly that it actually ran.
    let justChecked = $state(false);
    let wasChecking = false;
    $effect(() => {
        if (isChecking) {
            wasChecking = true;
            justChecked = false;
            return;
        }
        if (wasChecking) {
            wasChecking = false;
            justChecked = true;
            const timer = setTimeout(() => {
                justChecked = false;
            }, 2500);
            return () => clearTimeout(timer);
        }
    });
</script>

<div class="grid grid-cols-1 gap-4 sm:gap-5 lg:gap-6 pt-4 md:pt-0">
    {#if status === null}
        <div class="card flex flex-col items-center justify-center gap-5 px-6 py-16 text-center">
            <div class="relative flex size-16 items-center justify-center">
                <div class="absolute inset-0 animate-spin rounded-full border-2 border-slate-200 border-t-primary dark:border-navy-500 dark:border-t-accent-light"></div>
            </div>
            <div class="space-y-1.5">
                <h2 class="text-base font-semibold text-slate-700 dark:text-navy-100 lg:text-lg">
                    Checking server support
                </h2>
                <p class="mx-auto max-w-sm text-sm leading-relaxed text-slate-500 dark:text-navy-300">
                    Confirming whether your BVC server supports Minecraft Bedrock features.
                </p>
            </div>
        </div>
    {:else if status === "disabled"}
        <div class="card flex flex-col items-center justify-center gap-5 px-6 py-16 text-center">
            <div class="flex size-16 items-center justify-center rounded-full bg-slate-100 dark:bg-navy-600">
                <svg xmlns="http://www.w3.org/2000/svg" class="size-7 text-slate-400 dark:text-navy-300" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.8" d="M18.364 18.364A9 9 0 005.636 5.636m12.728 12.728A9 9 0 015.636 5.636m12.728 12.728L5.636 5.636"/>
                </svg>
            </div>
            <div class="space-y-1.5">
                <h2 class="text-base font-semibold text-slate-700 dark:text-navy-100 lg:text-lg">
                    Not supported on this server
                </h2>
                <p class="mx-auto max-w-sm text-sm leading-relaxed text-slate-500 dark:text-navy-300">
                    {#if serverHost}
                        <span class="font-medium text-slate-600 dark:text-navy-200">{serverHost}</span>
                        has Minecraft Bedrock support turned off, so this feature can't
                        run here. Connect to a BVC server with Bedrock support enabled
                        to use it.
                    {:else}
                        The BVC server you're connected to has Minecraft Bedrock support
                        turned off, so this feature can't run here. Connect to a BVC server
                        with Bedrock support enabled to use it.
                    {/if}
                </p>
            </div>
            <div class="flex flex-col items-center gap-3">
                <a
                    href="/server"
                    class="btn bg-primary font-medium text-white hover:bg-primary-focus dark:bg-accent dark:hover:bg-accent-focus"
                >
                    Switch server
                </a>
                <button
                    class="flex items-center gap-1.5 text-xs text-slate-400 underline decoration-dotted underline-offset-2 hover:text-slate-600 dark:text-navy-300 dark:hover:text-navy-100 disabled:no-underline"
                    onclick={onRetry}
                    disabled={isChecking}
                >
                    {#if isChecking}
                        <span class="inline-block size-3 animate-spin rounded-full border border-slate-300 border-t-transparent dark:border-navy-400"></span>
                        Checking…
                    {:else if justChecked}
                        Checked — still not supported
                    {:else}
                        Re-check this server
                    {/if}
                </button>
            </div>
        </div>
    {:else}
        <div class="card flex flex-col items-center justify-center gap-5 px-6 py-16 text-center">
            <div class="flex size-16 items-center justify-center rounded-full bg-warning/10">
                <svg xmlns="http://www.w3.org/2000/svg" class="size-7 text-warning" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.8" d="M12 9v3.75m0 3.75h.008v.008H12v-.008zM21 12a9 9 0 11-18 0 9 9 0 0118 0z"/>
                </svg>
            </div>
            <div class="space-y-1.5">
                <h2 class="text-base font-semibold text-slate-700 dark:text-navy-100 lg:text-lg">
                    Can't confirm server support
                </h2>
                <p class="mx-auto max-w-sm text-sm leading-relaxed text-slate-500 dark:text-navy-300">
                    We couldn't reach your BVC server to check whether it supports
                    Minecraft Bedrock features. We'll keep retrying automatically.
                </p>
            </div>
            <div class="flex flex-col items-center gap-2">
                <button
                    class="btn flex items-center gap-2 bg-primary font-medium text-white hover:bg-primary-focus dark:bg-accent dark:hover:bg-accent-focus disabled:opacity-60"
                    onclick={onRetry}
                    disabled={isChecking}
                >
                    {#if isChecking}
                        <span class="inline-block size-4 animate-spin rounded-full border-2 border-white/40 border-t-white"></span>
                        Checking…
                    {:else}
                        Retry now
                    {/if}
                </button>
                {#if justChecked && !isChecking}
                    <span class="text-xs text-slate-400 dark:text-navy-300">
                        Checked — still can't reach the server
                    </span>
                {/if}
            </div>
        </div>
    {/if}
</div>
