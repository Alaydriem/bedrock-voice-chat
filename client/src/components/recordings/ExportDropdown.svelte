<script lang="ts">
    import { onMount } from 'svelte';

    export let sessionId: string;
    export let onExport: (sessionId: string, withSpatial: boolean) => Promise<void>;
    export let onDelete: (sessionId: string) => Promise<void>;

    let isLoading = false;
    let wrapperRef: HTMLDivElement;
    let popper: any = null;

    async function handleExport(withSpatial: boolean) {
        isLoading = true;
        if (popper) popper.closePopper();
        try {
            await onExport(sessionId, withSpatial);
        } finally {
            isLoading = false;
        }
    }

    async function handleDelete() {
        if (confirm('Are you sure you want to delete this recording? This action cannot be undone.')) {
            isLoading = true;
            if (popper) popper.closePopper();
            try {
                await onDelete(sessionId);
            } finally {
                isLoading = false;
            }
        }
    }

    onMount(() => {
        // Wait for app initialization if needed
        const initializePopper = () => {
            console.log('Checking for Popper...', { Popper: (window as any).Popper });

            if (wrapperRef && typeof (window as any).Popper !== 'undefined') {
                console.log('Initializing Popper...');
                const config = {
                    placement: 'bottom-end',
                    modifiers: [
                        {
                            name: 'offset',
                            options: {
                                offset: [0, 4],
                            },
                        },
                    ],
                };

                popper = new (window as any).Popper(
                    wrapperRef,
                    '[data-popper-ref]',
                    '[data-popper-box]',
                    config
                );

                console.log('Popper created:', popper);
                return true;
            }
            return false;
        };

        // Try immediately
        if (!initializePopper()) {
            // If not available, wait for app:mounted event
            const onAppMounted = () => {
                setTimeout(() => {
                    if (!initializePopper()) {
                        console.log('Popper still not available after app mounted');
                    }
                }, 100);
            };

            window.addEventListener('app:mounted', onAppMounted, { once: true });

            // Also try after a short delay
            setTimeout(() => {
                if (!popper) {
                    initializePopper();
                }
            }, 500);
        }

        return () => {
            if (popper) {
                popper = null;
            }
        };
    });
</script>

<div bind:this={wrapperRef} class="relative">
    <button
        data-popper-ref
        class="btn size-8 rounded-full p-0 hover:bg-slate-300/20 focus:bg-slate-300/20 active:bg-slate-300/25 dark:hover:bg-navy-300/20 dark:focus:bg-navy-300/20 dark:active:bg-navy-300/25"
        class:opacity-50={isLoading}
        disabled={isLoading}
        aria-label="Export options"
    >
        {#if isLoading}
            <div class="spinner size-4 animate-spin rounded-full border-2 border-slate-400/20 border-t-slate-400"></div>
        {:else}
            <svg xmlns="http://www.w3.org/2000/svg" class="size-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 5v.01M12 12v.01M12 19v.01M12 6a1 1 0 110-2 1 1 0 010 2zm0 7a1 1 0 110-2 1 1 0 010 2zm0 7a1 1 0 110-2 1 1 0 010 2z"/>
            </svg>
        {/if}
    </button>

    <div data-popper-box class="min-w-[180px] rounded-lg border border-slate-150 bg-white py-1.5 font-inter shadow-lg dark:border-navy-500 dark:bg-navy-700 opacity-0 invisible transition-all duration-200 absolute z-50" style="transform: translate3d(0, 0, 0);">
        <ul>
            <li>
                <button
                    class="flex w-full items-center space-x-2 px-3 py-2 text-left text-slate-600 transition-colors hover:bg-slate-100 hover:text-slate-800 dark:text-navy-300 dark:hover:bg-navy-600 dark:hover:text-navy-100"
                    on:click={() => handleExport(true)}
                >
                    <svg xmlns="http://www.w3.org/2000/svg" class="size-4.5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M7 16a4 4 0 01-.88-7.903A5 5 0 1115.9 6L16 6a5 5 0 011 9.9M9 19l3 3m0 0l3-3m-3 3V10"/>
                    </svg>
                    <span>Export with Spatial</span>
                </button>
            </li>
            <li>
                <button
                    class="flex w-full items-center space-x-2 px-3 py-2 text-left text-slate-600 transition-colors hover:bg-slate-100 hover:text-slate-800 dark:text-navy-300 dark:hover:bg-navy-600 dark:hover:text-navy-100"
                    on:click={() => handleExport(false)}
                >
                    <svg xmlns="http://www.w3.org/2000/svg" class="size-4.5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M7 16a4 4 0 01-.88-7.903A5 5 0 1115.9 6L16 6a5 5 0 011 9.9M9 19l3 3m0 0l3-3m-3 3V10"/>
                    </svg>
                    <span>Export without Spatial</span>
                </button>
            </li>
            <li>
                <hr class="my-1 border-slate-150 dark:border-navy-500">
            </li>
            <li>
                <button
                    class="flex w-full items-center space-x-2 px-3 py-2 text-left text-error transition-colors hover:bg-error/10"
                    on:click={handleDelete}
                >
                    <svg xmlns="http://www.w3.org/2000/svg" class="size-4.5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16"/>
                    </svg>
                    <span>Delete Recording</span>
                </button>
            </li>
        </ul>
    </div>
</div>

<style>
    :global([data-popper-box].show) {
        opacity: 1 !important;
        visibility: visible !important;
    }
</style>