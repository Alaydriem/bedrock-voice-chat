<script lang="ts">
    import "../../../css/app.css";
    import { onMount } from 'svelte';
    import Onboarding from '../../../js/app/onboarding';

    let onboarding: Onboarding;

    onMount(async () => {
        onboarding = new Onboarding();
        await onboarding.initialize();

        // If already complete, navigate to next
        const state = onboarding.getCurrentState();
        if (state.welcome) {
            await onboarding.navigateToNext();
            return;
        }

        onboarding.preloader();
    });

    async function handleGetStarted() {
        await onboarding.completeStep('welcome');
        await onboarding.navigateToNext();
    }
</script>

<main class="grid w-full place-items-center min-h-dvh bg-slate-50 dark:bg-navy-900 p-4">
    <div class="card w-full max-w-md p-8">
        <div class="flex justify-center mb-8">
            <img src="/images/app-logo-transparent.png" alt="Bedrock Voice Chat" class="h-20" />
        </div>

        <div class="text-center">
            <h1 class="text-3xl font-bold mb-4 text-slate-900 dark:text-navy-50">
                Welcome to Bedrock Voice Chat
            </h1>
            <p class="text-slate-600 dark:text-navy-200 mb-6 leading-relaxed">
                Proximity and Group Voice Chat with your friends in Minecraft Bedrock Edition.
            </p>

            <div class="text-left space-y-3 mb-8">
                <div class="flex items-start">
                    <i class="fas fa-check-circle text-success mt-1 mr-3"></i>
                    <div>
                        <p class="font-semibold text-slate-800 dark:text-navy-100">Proximity & Group Chat</p>
                        <p class="text-sm text-slate-600 dark:text-navy-300">Hear those near you or join a group for persistent communication</p>
                    </div>
                </div>
                <div class="flex items-start">
                    <i class="fas fa-check-circle text-success mt-1 mr-3"></i>
                    <div>
                        <p class="font-semibold text-slate-800 dark:text-navy-100">Record Your Sessions</p>
                        <p class="text-sm text-slate-600 dark:text-navy-300">Record your audion sessions with a click of a button, then export each audio track for use later.</p>
                    </div>
                </div>
                <div class="flex items-start">
                    <i class="fas fa-check-circle text-success mt-1 mr-3"></i>
                    <div>
                        <p class="font-semibold text-slate-800 dark:text-navy-100">Cross-Platform</p>
                        <p class="text-sm text-slate-600 dark:text-navy-300">Works on Desktop and Mobile for Console users</p>
                    </div>
                </div>
            </div>
        </div>

        <button
            on:click={handleGetStarted}
            class="btn w-full bg-primary hover:bg-primary-focus focus:bg-primary-focus active:bg-primary-focus/90 dark:bg-accent dark:hover:bg-accent-focus dark:focus:bg-accent-focus dark:active:bg-accent/90 text-white font-semibold py-3"
        >
            Get Started
        </button>
    </div>
</main>
