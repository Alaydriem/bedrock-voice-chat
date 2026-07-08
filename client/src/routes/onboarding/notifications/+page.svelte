<script lang="ts">
    import "../../../css/app.css";
    import { onMount, onDestroy } from 'svelte';
    import Onboarding from '../../../js/app/onboarding';
    import { PermissionType } from 'tauri-plugin-audio-permissions';
    import PermissionRequestManager, { type PermissionFlowState } from '../../../js/app/PermissionRequestManager';

    const STATUS_PHRASES = [
        'Opening the permission prompt…',
        'Waiting for your response…',
        'Confirming notification access…',
        'Almost there…',
    ];
    const BRAILLE_FRAMES = ['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];

    let onboarding: Onboarding | null = null;
    let manager: PermissionRequestManager | null = null;
    let unsub: (() => void) | null = null;
    let completed = false;

    let flowState = $state<PermissionFlowState>('idle');
    let phraseIndex = $state(0);
    let spinnerFrame = $state(0);

    onMount(async () => {
        onboarding = new Onboarding();
        await onboarding.initialize();

        if (onboarding.getCurrentState().notifications) {
            await onboarding.navigateToNext();
            return;
        }

        manager = new PermissionRequestManager(PermissionType.Notification);
        unsub = manager.state.subscribe((s) => {
            flowState = s;
            if (s === 'granted') void handleGranted();
        });
        await manager.start();
        onboarding.preloader();
    });

    onDestroy(() => {
        unsub?.();
        manager?.destroy();
    });

    async function handleGranted() {
        if (completed || !onboarding) return;
        completed = true;
        await onboarding.completeStep('notifications');
        setTimeout(() => onboarding?.navigateToNext(), 500);
    }

    function handleRequest() {
        void manager?.requestPermission();
    }

    function handleCancel() {
        manager?.cancel();
    }

    $effect(() => {
        if (flowState !== 'requesting') return;
        phraseIndex = 0;
        const phraseId = setInterval(() => {
            phraseIndex = (phraseIndex + 1) % STATUS_PHRASES.length;
        }, 1600);

        spinnerFrame = 0;
        const reduceMotion = window.matchMedia('(prefers-reduced-motion: reduce)').matches;
        const spinnerId = reduceMotion ? null : setInterval(() => {
            spinnerFrame = (spinnerFrame + 1) % BRAILLE_FRAMES.length;
        }, 90);

        return () => {
            clearInterval(phraseId);
            if (spinnerId) clearInterval(spinnerId);
        };
    });
</script>

<main class="flex w-full h-full overflow-y-auto bg-slate-50 dark:bg-navy-900 p-4">
    <div class="card w-full max-w-md p-8 text-center m-auto">
        {#if flowState === 'granted'}
            <div class="bvc-badge bvc-badge--success mx-auto"><i class="fa-solid fa-check"></i></div>
            <h1 class="mt-4 text-2xl font-semibold text-slate-900 dark:text-navy-50">
                Notifications Enabled
            </h1>
            <p class="mt-2 text-slate-600 dark:text-navy-200">
                Great! You're all set!
            </p>
        {:else if flowState === 'requesting'}
            <div class="bvc-ring mx-auto" role="status" aria-live="polite"></div>
            <h1 class="mt-4 text-2xl font-semibold text-slate-900 dark:text-navy-50">
                Requesting Notification Access
            </h1>
            <div class="mt-4 flex items-center justify-center gap-2.5 min-h-[22px] font-inter">
                <span class="bvc-spinner text-primary dark:text-accent-light" aria-hidden="true">{BRAILLE_FRAMES[spinnerFrame]}</span>
                <span class="bvc-shimmer text-xs+">{STATUS_PHRASES[phraseIndex]}</span>
            </div>
            <button
                type="button"
                class="mt-5 text-tiny+ text-slate-400 hover:text-slate-500 hover:underline dark:text-navy-300 dark:hover:text-navy-200"
                onclick={handleCancel}
            >
                Cancel
            </button>
        {:else}
            <div class="flex justify-center mb-6">
                <div class="flex items-center justify-center w-20 h-20 rounded-full bg-slate-200 dark:bg-navy-700">
                    <i class="fas fa-bell-slash text-slate-600 dark:text-navy-300 text-3xl"></i>
                </div>
            </div>
            <h1 class="text-2xl font-semibold mb-4 text-slate-900 dark:text-navy-50">
                Enable Notifications
            </h1>
            <p class="text-slate-600 dark:text-navy-200 mb-6">
                BVC needs access to create a persistent notifications to allow for microphone recording to work when the app is not in the foreground or when your screen locks.
            </p>

            {#if flowState === 'denied'}
            <div class="alert bg-warning/10 text-warning border border-warning/20 rounded-lg p-4 mb-6 text-sm">
                <i class="fas fa-exclamation-triangle mr-2"></i>
                We didn't get access. Enable it in your device settings and we'll detect it automatically, or try again.
            </div>
            {/if}

            {#if flowState === 'error'}
            <div class="alert bg-error/10 text-error border border-error/20 rounded-lg p-4 mb-6 text-sm">
                <i class="fas fa-times-circle mr-2"></i>
                Something went wrong requesting permission. Please try again or check your device settings.
            </div>
            {/if}

            <button
                onclick={handleRequest}
                class="btn w-full bg-primary hover:bg-primary-focus focus:bg-primary-focus active:bg-primary-focus/90 dark:bg-accent dark:hover:bg-accent-focus dark:focus:bg-accent-focus dark:active:bg-accent/90 text-white font-semibold py-3"
            >
                <i class="fas fa-bell mr-2"></i>
                {flowState === 'denied' || flowState === 'error' ? 'Try again' : 'Enable Notifications'}
            </button>
        {/if}
    </div>
</main>
