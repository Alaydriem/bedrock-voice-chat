<script lang="ts">
    import { onMount } from "svelte";
    import { invoke } from "@tauri-apps/api/core";
    import AudioSettings from "../../../js/app/settings/audio";
    onMount(() => {
        const settings = new AudioSettings();
        settings.initialize();
    });
</script>

<div id="audio-settings-page" class="grid grid-cols-1 gap-4 sm:gap-5 lg:gap-6 hidden">
    <!-- Audio Settings -->
    <div class="card px-4 pb-4 sm:px-5">
        <div class="my-3 flex h-8 items-center justify-between">
            <h2
                class="font-medium tracking-wide text-slate-700 line-clamp-1 dark:text-navy-100 lg:text-base"
            >
                Audio Settings
            </h2>
        </div>

        <div class="flex mb-4 -mx-2">
            <div class="mt-5 flex-1 px-5" id="input-audio-device-container">
            </div>

            <div class="mt-5 flex-1 px-5" id="output-audio-device-container">
            </div>
        </div>
    </div>
</div>
