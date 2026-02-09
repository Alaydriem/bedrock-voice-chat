<script lang="ts">
    import { invoke } from "@tauri-apps/api/core";
    import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow';
    import { Store } from '@tauri-apps/plugin-store';
    import { info, error as logError } from '@tauri-apps/plugin-log';
    import { onMount, onDestroy } from 'svelte';
    import type { KeybindConfig } from "../../../js/bindings/KeybindConfig.ts";
    import type { MuteEvent } from "../../../js/bindings/MuteEvent.ts";
    import type { PttEvent } from "../../../js/bindings/PttEvent.ts";

    // Props
    export let disabled: boolean = false;

    // Internal state
    let isMuted = false;
    let isToggling = false;
    let isPttMode = false;
    let pttActive = false;
    let pttKeyLabel = "";
    let unlistenMuteInput: (() => void) | null = null;
    let unlistenPttActive: (() => void) | null = null;

    // Reactive classes
    $: iconClass = isPttMode
        ? (pttActive ? "fa-solid fa-microphone text-success" : "fa-solid fa-microphone-slash text-slate-400")
        : (isMuted ? "fa-solid fa-microphone-slash text-error" : "fa-solid fa-microphone");

    $: tooltipText = isPttMode
        ? `Hold ${pttKeyLabel || "key"} to speak`
        : "Toggle Mute";

    function displayKey(combo: string): string {
        if (!combo) return "key";
        return combo.split("+").map(part => {
            if (part.startsWith("Key") && part.length === 4) return part.charAt(3);
            if (part.startsWith("Digit") && part.length === 6) return part.charAt(5);
            if (part === "ShiftLeft") return "Shift";
            if (part === "ControlLeft") return "Ctrl";
            if (part === "Backquote") return "`";
            if (part === "BracketLeft") return "[";
            if (part === "BracketRight") return "]";
            if (part === "Backslash") return "\\";
            return part;
        }).join("+");
    }

    const handleToggleMute = async () => {
        if (isPttMode || isToggling) return;

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

        // Load PTT mode from store
        const store = await Store.load("store.json", { autoSave: false });
        const keybinds = await store.get<KeybindConfig>("keybinds");
        if (keybinds) {
            isPttMode = keybinds.voiceMode === "pushToTalk";
            pttKeyLabel = displayKey(keybinds.pushToTalk || "Backquote");
        }

        // Listen for mute state changes from WebSocket or other sources
        const muteInputEvent: MuteEvent = "mute:input";
        const appWebview = getCurrentWebviewWindow();
        unlistenMuteInput = await appWebview.listen(muteInputEvent, (event: any) => {
            isMuted = event.payload as boolean;
            info(`Microphone mute state changed: ${isMuted ? 'muted' : 'unmuted'}`);
        });

        // Listen for PTT active state
        const pttActiveEvent: PttEvent = "ptt:active";
        unlistenPttActive = await appWebview.listen(pttActiveEvent, (event: any) => {
            pttActive = event.payload as boolean;
        });
    });

    onDestroy(() => {
        if (unlistenMuteInput) {
            unlistenMuteInput();
        }
        if (unlistenPttActive) {
            unlistenPttActive();
        }
    });
</script>

<button
    class="btn size-8 rounded-full p-0 hover:bg-slate-300/20 focus:bg-slate-300/20 active:bg-slate-300/25 dark:hover:bg-navy-300/20 dark:focus:bg-navy-300/20 dark:active:bg-navy-300/25"
    class:opacity-50={isToggling}
    class:animate-pulse={isPttMode && pttActive}
    onclick={handleToggleMute}
    disabled={disabled || isToggling || isPttMode}
    data-tooltip={tooltipText}
    aria-label={tooltipText}
    title={tooltipText}
>
    <i class={iconClass}></i>
</button>
