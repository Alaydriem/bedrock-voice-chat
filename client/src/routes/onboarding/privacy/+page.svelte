<script lang="ts">
    import "../../../css/app.css";
    import { onMount } from 'svelte';
    import Onboarding from '../../../js/app/onboarding';

    let onboarding: Onboarding;

    onMount(async () => {
        onboarding = new Onboarding();
        await onboarding.initialize();

        const state = onboarding.getCurrentState();
        if (state.privacy) {
            await onboarding.navigateToNext();
            return;
        }

        onboarding.preloader();
    });

    async function handleContinue() {
        await onboarding.completeStep('privacy');
        await onboarding.navigateToNext();
    }
</script>

<main class="grid w-full place-items-center min-h-dvh bg-slate-50 dark:bg-navy-900 p-4">
    <div class="card w-full max-w-md p-8">
        <div class="flex justify-center mb-8">
            <div class="flex h-20 w-20 items-center justify-center rounded-full bg-primary/10 dark:bg-accent/10">
                <svg xmlns="http://www.w3.org/2000/svg" class="h-10 w-10 text-primary dark:text-accent" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12l2 2 4-4m5.618-4.016A11.955 11.955 0 0112 2.944a11.955 11.955 0 01-8.618 3.04A12.02 12.02 0 003 9c0 5.591 3.824 10.29 9 11.622 5.176-1.332 9-6.03 9-11.622 0-1.042-.133-2.052-.382-3.016z"/>
                </svg>
            </div>
        </div>

        <div class="text-center">
            <h1 class="text-3xl font-bold mb-4 text-slate-900 dark:text-navy-50">
                You're in Control
            </h1>
            <p class="text-slate-600 dark:text-navy-200 mb-6 leading-relaxed">
                We use logging and analytics tools to measure performance and capture errors — this helps us fix bugs and improve your experience.
            </p>

            <div class="text-left space-y-3 mb-8">
                <div class="flex items-start">
                    <i class="fas fa-chart-line text-primary dark:text-accent mt-1 mr-3"></i>
                    <div>
                        <p class="font-semibold text-slate-800 dark:text-navy-100">Performance Monitoring</p>
                        <p class="text-sm text-slate-600 dark:text-navy-300">Anonymous usage data helps us identify and fix slowdowns</p>
                    </div>
                </div>
                <div class="flex items-start">
                    <i class="fas fa-bug text-primary dark:text-accent mt-1 mr-3"></i>
                    <div>
                        <p class="font-semibold text-slate-800 dark:text-navy-100">Error Reporting</p>
                        <p class="text-sm text-slate-600 dark:text-navy-300">Crash reports let us resolve issues before they affect more users</p>
                    </div>
                </div>
                <div class="flex items-start">
                    <i class="fas fa-toggle-on text-primary dark:text-accent mt-1 mr-3"></i>
                    <div>
                        <p class="font-semibold text-slate-800 dark:text-navy-100">Your Choice</p>
                        <p class="text-sm text-slate-600 dark:text-navy-300">You can turn this off at any time in Settings → About</p>
                    </div>
                </div>
            </div>
        </div>

        <button
            onclick={handleContinue}
            class="btn w-full bg-primary hover:bg-primary-focus focus:bg-primary-focus active:bg-primary-focus/90 dark:bg-accent dark:hover:bg-accent-focus dark:focus:bg-accent-focus dark:active:bg-accent/90 text-white font-semibold py-3"
        >
            Continue
        </button>
    </div>
</main>
