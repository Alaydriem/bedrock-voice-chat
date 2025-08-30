<script lang="ts">
    import { onMount } from "svelte";
    import AudioSettings from "../../../js/app/settings/audio";
    import { family } from '@tauri-apps/plugin-os';
    
    let osFamily: string = "";
    onMount(async () => {
        osFamily = await family();
        const settings = new AudioSettings();
        settings.initialize(osFamily);
    });
</script>

<div id="audio-settings-page" class="grid grid-cols-1 gap-4 sm:gap-5 lg:gap-6 hidden">
    <!-- Audio Settings -->
    <div class="card px- pb-4 sm:px-5">
        <div class="my-3 flex h-8n flex-col">
            <h2
                class="font-medium tracking-wide text-slate-700 line-clamp-1 dark:text-navy-100 lg:text-base pb-2"
            >
                Audio Settings
            </h2>
            <p class="text-sm leading-6">
                Configure your audio input and audio devices. Ensure both devices are configured to run at 48Khz Stereo.
            </p>
        </div>

        {#if osFamily !== "ios" && osFamily !== "android"}
            <div class="flex mb-4 -mx-2">
                
                <div class="mt-5 flex-1 px-5" id="input-audio-device-container">
                </div>
                <div
                    id="audio-device-select-spinner"
                    class="justify-center spinner size-7 animate-spin rounded-full border-[3px] border-warning/30 border-r-warning"
                ></div>

                <div class="mt-5 flex-1 px-5" id="output-audio-device-container">
                </div>
            </div>
        {/if}

        <div class="my-4 h-px  bg-slate-200 dark:bg-navy-500"></div>

        <div class="my-3 flex h-8n flex-col">
            <h2
                class="font-medium tracking-wide text-slate-700 line-clamp-1 dark:text-navy-100 lg:text-base pb-2"
            >
                Noise Suppression
            </h2>
            <p class="text-sm leading-6">
                Suppress background noise from your microphone.
            </p>
        </div>

        <div class="flex mb-4 -mx-2 flex-col">
            <label class="inline-flex items-center space-x-2 pb-2">
                <input
                disabled
                id="noise-suppression-rs-toggle"
                class="form-switch h-5 w-10 rounded-full bg-slate-300 before:rounded-full before:bg-slate-50 checked:bg-primary checked:before:bg-white dark:bg-navy-900 dark:before:bg-navy-300 dark:checked:bg-accent dark:checked:before:bg-white"
                type="checkbox"
                />
                <span x-tooltip.light="'A standard noise gate modeled after OBS\' Noise Gate Filter. Effective, but requires manual tuning for your environment.'">Noise Gate RS</span>
            </label>

            <div id="noise-gate-audio-controls" class="hidden pt-5 pb-5 flex flex-row justify-evenly">

            </div>
            <label class="inline-flex items-center space-x-2 pb-2 pt-2">
                <input
                disabled
                class="form-switch h-5 w-10 rounded-full bg-slate-300 before:rounded-full before:bg-slate-50 checked:bg-primary checked:before:bg-white dark:bg-navy-900 dark:before:bg-navy-300 dark:checked:bg-accent dark:checked:before:bg-white"
                type="checkbox"
                />
                <span x-tooltip.light="'Experimental. A more advanced filtering neural network.'">Deep Filter Net</span>
            </label>
        </div>
    </div>
</div>
