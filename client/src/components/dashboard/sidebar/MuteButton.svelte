<script lang="ts">
    import { invoke } from "@tauri-apps/api/core";
    import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow';
    import { info, error as logError } from '@tauri-apps/plugin-log';
    import { onMount, onDestroy } from 'svelte';

    // Props
    export let disabled: boolean = false;

    // Internal state
    let isMuted = false;
    let isToggling = false;
    let unlistenMuteInput: (() => void) | null = null;

    // Reactive classes
    $: iconClass = isMuted ? "fa-solid fa-microphone-slash text-error" : "fa-solid fa-microphone";

    const handleToggleMute = async () => {
        if (isToggling) return;

        try {
            isToggling = true;

            await invoke('mute', { device: 'InputDevice' });

            // Toggle the local state
            isMuted = !isMuted;

            info(`Microphone ${isMuted ? 'muted' : 'unmuted'}`);

        } catch (error) {
            logError(`Failed to toggle microphone mute: ${error}`);
        } finally {
            isToggling = false;
        }
    };

    const loadMuteStatus = async () => {
        try {
            isMuted = await invoke('mute_status', { device: 'InputDevice' }) as boolean;
        } catch (error) {
            logError(`Failed to get microphone mute status: ${error}`);
        }
    };

    onMount(async () => {
        await loadMuteStatus();

        // Listen for mute state changes from WebSocket or other sources
        const appWebview = getCurrentWebviewWindow();
        unlistenMuteInput = await appWebview.listen('mute:input', (event: any) => {
            isMuted = event.payload as boolean;
            info(`Microphone mute state changed: ${isMuted ? 'muted' : 'unmuted'}`);
        });
    });

    onDestroy(() => {
        if (unlistenMuteInput) {
            unlistenMuteInput();
        }
    });
</script>

<button
    class="btn size-8 rounded-full p-0 hover:bg-slate-300/20 focus:bg-slate-300/20 active:bg-slate-300/25 dark:hover:bg-navy-300/20 dark:focus:bg-navy-300/20 dark:active:bg-navy-300/25"
    class:opacity-50={isToggling}
    onclick={handleToggleMute}
    disabled={disabled || isToggling}
    data-tooltip="Toggle Mute"
    aria-label="Toggle Mute"
    title="Mute Microphone"
>
    <i class={iconClass}></i>
</button>