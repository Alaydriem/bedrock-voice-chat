<script lang="ts">
    import "../../../css/app.css";
    import { onMount } from 'svelte';
    import Onboarding from '../../../js/app/onboarding';
    import {
        checkPermission,
        requestPermission,
        PermissionType,
        type PermissionResponse
    } from 'tauri-plugin-audio-permissions';

    let onboarding: Onboarding;
    let permissionGranted = false;
    let permissionDenied = false;
    let isChecking = false;

    onMount(async () => {
        onboarding = new Onboarding();
        await onboarding.initialize();

        const state = onboarding.getCurrentState();
        if (state.notifications) {
            await onboarding.navigateToNext();
            return;
        }

        await checkCurrentPermission();

        // If permission already granted, auto-complete and proceed
        if (permissionGranted) {
            await onboarding.completeStep('notifications');
            await onboarding.navigateToNext();
            return;
        }

        onboarding.preloader();
    });

    async function checkCurrentPermission() {
        try {
            const response: PermissionResponse = await checkPermission({
                permissionType: PermissionType.Notification
            });
            permissionGranted = response.granted;
        } catch (error) {
            console.error('Error checking notification permission:', error);
            permissionGranted = false;
        }
    }

    async function handleRequestPermission() {
        isChecking = true;
        permissionDenied = false;

        try {
            const response: PermissionResponse = await requestPermission({
                permissionType: PermissionType.Notification
            });

            if (response.granted) {
                permissionGranted = true;
                await onboarding.completeStep('notifications');
                setTimeout(() => {
                    onboarding.navigateToNext();
                }, 500);
            } else {
                permissionDenied = true;
            }
        } catch (error) {
            console.error('Error requesting notification permission:', error);
            permissionDenied = true;
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
                {permissionGranted ? 'Notifications Enabled' : 'Enable Notifications'}
            </h1>
            <p class="text-slate-600 dark:text-navy-200 mb-6">
                {#if permissionGranted}
                    Great! You're all set!
                {:else}
                    BVC needs access to create a persistent notifications to allow for microphone recording to work when the app is not in the foreground or when your screen locks.
                {/if}
            </p>

            {#if permissionDenied}
            <div class="alert bg-warning/10 text-warning border border-warning/20 rounded-lg p-4 mb-6 text-sm">
                <i class="fas fa-exclamation-triangle mr-2"></i>
                Please enable notifications in your device settings to continue.
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
                Enable Notifications
            {/if}
        </button>
    </div>
</main>
