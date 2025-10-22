<script lang="ts">
    import { invoke } from "@tauri-apps/api/core";
    import { info, error as logError } from '@tauri-apps/plugin-log';
    import { onMount } from 'svelte';

    // Props
    export let disabled: boolean = false;

    // Internal state
    let isDeafened = false;
    let isToggling = false;

    // Reactive classes
    $: iconClass = isDeafened ? "fa-solid fa-volume-xmark text-error" : "fa-solid fa-volume-high";

    const handleToggleDeafen = async () => {
        if (isToggling) return;

        try {
            isToggling = true;

            await invoke('mute', { device: 'OutputDevice' });

            // Toggle the local state
            isDeafened = !isDeafened;

            info(`Audio output ${isDeafened ? 'deafened' : 'enabled'}`);

        } catch (error) {
            logError(`Failed to toggle audio output mute: ${error}`);
        } finally {
            isToggling = false;
        }
    };

    const loadDeafenStatus = async () => {
        try {
            isDeafened = await invoke('mute_status', { device: 'OutputDevice' }) as boolean;
        } catch (error) {
            logError(`Failed to get audio output mute status: ${error}`);
        }
    };

    onMount(() => {
        loadDeafenStatus();
    });
</script>

<button
    class="btn size-8 rounded-full p-0 hover:bg-slate-300/20 focus:bg-slate-300/20 active:bg-slate-300/25 dark:hover:bg-navy-300/20 dark:focus:bg-navy-300/20 dark:active:bg-navy-300/25"
    class:opacity-50={isToggling}
    on:click={handleToggleDeafen}
    disabled={disabled || isToggling}
    data-tooltip="Toggle Deafen"
    aria-label="Toggle Deafen"
    title="Mute Audio Output"
>
    <i class={iconClass}></i>
</button>