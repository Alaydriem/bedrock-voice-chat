<script lang="ts">
    import "../../../css/app.css";
    import { onMount } from 'svelte';
    import Onboarding from '../../../js/app/onboarding';
    import { PermissionType } from 'tauri-plugin-audio-permissions';
    import { checkPermissionStatus, requestPermissionWithTimeout } from '../../../js/app/utils/permissionHelpers';

    let onboarding: Onboarding;
    let permissionGranted = false;
    let permissionDenied = false;
    let isChecking = false;
    let permissionError = false;

    onMount(async () => {
        onboarding = new Onboarding();
        await onboarding.initialize();

        const state = onboarding.getCurrentState();
        if (state.microphone) {
            await onboarding.navigateToNext();
            return;
        }

        // Check current permission status
        await checkCurrentPermission();

        // If permission already granted, auto-complete and proceed
        if (permissionGranted) {
            await onboarding.completeStep('microphone');
            await onboarding.navigateToNext();
            return;
        }

        onboarding.preloader();
    });

    async function checkCurrentPermission() {
        try {
            const response = await checkPermissionStatus(PermissionType.Audio);
            permissionGranted = response.granted;
        } catch (error) {
            console.error('Error checking microphone permission:', error);
            permissionGranted = false;
        }
    }

    async function handleRequestPermission() {
        isChecking = true;
        permissionDenied = false;
        permissionError = false;

        try {
            const response = await requestPermissionWithTimeout(
                PermissionType.Audio,
                10000 // 10 second timeout
            );

            if (response.granted) {
                permissionGranted = true;
                await onboarding.completeStep('microphone');
                setTimeout(() => {
                    onboarding.navigateToNext();
                }, 500);
            } else {
                permissionDenied = true;
            }
        } catch (error) {
            console.error('Error requesting microphone permission:', error);

            // On timeout or error, re-check permission status in case it was actually granted
            try {
                const statusCheck = await checkPermissionStatus(PermissionType.Audio);
                if (statusCheck.granted) {
                    // Permission was actually granted, proceed
                    permissionGranted = true;
                    await onboarding.completeStep('microphone');
                    setTimeout(() => {
                        onboarding.navigateToNext();
                    }, 500);
                    return;
                }
            } catch (recheckError) {
                console.error('Error rechecking permission status:', recheckError);
            }

            // If we get here, permission was not granted
            if (error instanceof Error && error.message.includes('timeout')) {
                permissionError = true;
            } else {
                permissionDenied = true;
            }
        } finally {
            isChecking = false;
        }
    }
</script>

<main class="grid w-full place-items-center min-h-dvh bg-slate-50 dark:bg-navy-900 p-4">
    <div class="card w-full max-w-md p-8">
        <div class="flex justify-center mb-6">
            <div class="flex items-center justify-center w-20 h-20 rounded-full {permissionGranted ? 'bg-success/10' : 'bg-slate-200 dark:bg-navy-700'}">
                <i class="fas {permissionGranted ? 'fa-bell text-success' : 'fa-bell-slash text-slate-600 dark:text-navy-300'} text-3xl"></i>
            </div>
        </div>

        <div class="text-center">
            <h1 class="text-2xl font-semibold mb-4 text-slate-900 dark:text-navy-50">
                {permissionGranted ? 'Microphone Access Granted' : 'Microphone Access Required'}
            </h1>
            <p class="text-slate-600 dark:text-navy-200 mb-6">
                {#if permissionGranted}
                    You're all set! Voice chat requires microphone access to transmit your audio.
                {:else}
                    To use voice chat, we need permission to access your microphone. Your audio is only transmitted when you're speaking and not muted
                {/if}
            </p>

            {#if permissionDenied}
            <div class="alert bg-warning/10 text-warning border border-warning/20 rounded-lg p-4 mb-6 text-sm">
                <i class="fas fa-exclamation-triangle mr-2"></i>
                Please enable audio recording in your device settings to continue.
            </div>
            {/if}

            {#if permissionError}
            <div class="alert bg-error/10 text-error border border-error/20 rounded-lg p-4 mb-6 text-sm">
                <i class="fas fa-times-circle mr-2"></i>
                Permission request timed out. Please try again or check your device settings.
            </div>
            {/if}
        </div>

        <button
            on:click={handleRequestPermission}
            disabled={isChecking}
            class="btn w-full bg-primary hover:bg-primary-focus focus:bg-primary-focus active:bg-primary-focus/90 dark:bg-accent dark:hover:bg-accent-focus dark:focus:bg-accent-focus dark:active:bg-accent/90 text-white font-semibold py-3 disabled:opacity-50"
        >
            {#if isChecking}
                <span class="spinner h-5 w-5 mr-2"></span>
                Checking...
            {:else}
                <i class="fas fa-bell mr-2"></i>
                Grant Microphone Access
            {/if}
        </button>
    </div>
</main>
