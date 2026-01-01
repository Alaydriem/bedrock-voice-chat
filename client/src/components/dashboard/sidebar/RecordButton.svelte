<script lang="ts">
    import { invoke } from "@tauri-apps/api/core";
    import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow';
    import { info, error as logError } from '@tauri-apps/plugin-log';
    import { onMount, onDestroy } from "svelte";
    import PlatformDetector from '../../../js/app/utils/PlatformDetector';

    // Props
    export let disabled: boolean = false;

    // Internal state
    let isRecording = false;
    let recordingError: string | null = null;
    let unlistenRecordingError: (() => void) | null = null;
    let unlistenStarted: (() => void) | null = null;
    let unlistenStopped: (() => void) | null = null;
    let isMobile = false;

    const platformDetector = new PlatformDetector();

    // Reactive button styles
    $: buttonClass = isRecording
        ? "btn bg-error font-medium text-white hover:bg-red-600 focus:bg-red-600 active:bg-red-600/80 animate-pulse"
        : "btn bg-slate-150 font-medium text-slate-800 hover:bg-slate-200 focus:bg-slate-200 active:bg-slate-200/80 dark:bg-navy-500 dark:text-navy-50 dark:hover:bg-navy-450 dark:focus:bg-navy-450 dark:active:bg-navy-450/90";

    $: iconClass = isRecording ? "fa-solid fa-stop" : "fa-solid fa-circle";
    $: buttonTitle = isRecording ? "Stop Recording" : "Start Recording";

    const handleRecordToggle = async () => {
        try {
            recordingError = null;

            if (isRecording) {
                // Stop recording
                await invoke('stop_recording');
                isRecording = false;
                info('Recording stopped');
            } else {
                // Start recording
                const sessionId = await invoke('start_recording');
                isRecording = true;
                info(`Recording started with session ID: ${sessionId}`);
            }

        } catch (error) {
            recordingError = `Failed to ${isRecording ? 'stop' : 'start'} recording: ${error}`;
            logError(recordingError);
        }
    };

    const clearError = () => {
        recordingError = null;
    };

    onMount(async () => {
        // Check if we're on mobile
        isMobile = await platformDetector.checkMobile();

        // Sync with backend truth on mount
        try {
            isRecording = await invoke('is_recording');
            if (isRecording) {
                info('Synced recording state: currently recording');
            }
        } catch (error) {
            logError(`Failed to query recording state: ${error}`);
        }

        // Listen for recording error events from Rust
        const appWebview = getCurrentWebviewWindow();
        unlistenRecordingError = await appWebview.listen('recording_error', (event: any) => {
            logError(`Recording error: ${event.payload.error}`);
            recordingError = `Recording failed: ${event.payload.error}`;
            isRecording = false; // Stop recording on error
        });

        // Listen to recording state changes
        unlistenStarted = await appWebview.listen('recording:started', (event: any) => {
            info(`Recording started event received: ${event.payload}`);
            isRecording = true;
        });

        unlistenStopped = await appWebview.listen('recording:stopped', () => {
            info('Recording stopped event received');
            isRecording = false;
        });
    });

    onDestroy(() => {
        if (unlistenRecordingError) {
            unlistenRecordingError();
        }
        if (unlistenStarted) {
            unlistenStarted();
        }
        if (unlistenStopped) {
            unlistenStopped();
        }
    });
</script>

<!-- Only show record button on desktop platforms -->
{#if !isMobile}
<div class="relative">
    <button
        class={buttonClass}
        onclick={handleRecordToggle}
        {disabled}
        title={buttonTitle}
    >
        <i class={iconClass}></i>
        <span class="ml-1">REC</span>
    </button>

    <!-- Error tooltip -->
    {#if recordingError}
        <div
            class="absolute bottom-full right-0 mb-2 w-48 rounded bg-error px-2 py-1 text-xs text-white shadow-lg z-50"
            role="alert"
        >
            {recordingError}
            <button
                class="ml-1 hover:text-red-200"
                onclick={clearError}
                aria-label="Dismiss error"
            >
                ×
            </button>
        </div>
    {/if}
</div>
{/if}