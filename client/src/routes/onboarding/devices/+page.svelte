<script lang="ts">
    import "../../../css/app.css";
    import { onMount } from 'svelte';
    import Onboarding from '../../../js/app/onboarding';
    import AudioDeviceSelector from '../../../components/audio/AudioDeviceSelector.svelte';
    import PlatformDetector from "../../../js/app/utils/PlatformDetector";

    let onboarding: Onboarding;

    onMount(async () => {
        onboarding = new Onboarding();
        await onboarding.initialize();

        const state = onboarding.getCurrentState();
        if (state.devices) {
            await onboarding.navigateToNext();
            return;
        }

        const platformDetector = new PlatformDetector();
        const isMobile = await platformDetector.checkMobile();
        if (isMobile) {
            handleContinue();
            return;
        }

        // Mobile devices are handled by +page.ts and never reach here
        onboarding.preloader();
    });

    async function handleContinue() {
        await onboarding.completeStep('devices');
        await onboarding.navigateToNext();
    }
</script>

<main class="grid w-full place-items-center min-h-dvh bg-slate-50 dark:bg-navy-900 p-4">
    <div class="card w-full max-w-2xl p-8">
        <div class="flex justify-center mb-6">
            <div class="flex items-center justify-center w-20 h-20 rounded-full bg-slate-200 dark:bg-navy-700">
                <i class="fas fa-headset text-slate-600 dark:text-navy-300 text-3xl"></i>
            </div>
        </div>

        <div class="text-center mb-8">
            <h1 class="text-2xl font-semibold mb-4 text-slate-900 dark:text-navy-50">
                Configure Audio Devices
            </h1>
            <p class="text-slate-600 dark:text-navy-200">
                Select your preferred input and output device. Make sure your device supports 48 kHz! (or 44.1 kHz on certain Android devices). You can change these later in settings.
            </p>
        </div>

        <div class="space-y-6">
            <AudioDeviceSelector />
        </div>

        <button
            on:click={handleContinue}
            class="btn w-full mt-8 bg-primary hover:bg-primary-focus focus:bg-primary-focus active:bg-primary-focus/90 dark:bg-accent dark:hover:bg-accent-focus dark:focus:bg-accent-focus dark:active:bg-accent/90 text-white font-semibold py-3"
        >
            <i class="fas fa-check-circle mr-2"></i>
            Complete Setup
        </button>
    </div>
</main>
