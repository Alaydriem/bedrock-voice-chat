<script lang="ts">
    import { onMount } from "svelte";
    import { Store } from '@tauri-apps/plugin-store';
    import AudioDeviceSelector from '../../audio/AudioDeviceSelector.svelte';
    import NoiseGateSettings from '../../audio/NoiseGateSettings.svelte';

    let store: Store | undefined = $state(undefined);
    let isReady = $state(false);

    onMount(async () => {
        store = await Store.load("store.json", {
            autoSave: false,
            defaults: {}
        });
        isReady = true;
    });
</script>

<div id="audio-settings-page" class="grid grid-cols-1 gap-4 sm:gap-5 lg:gap-6 pt-4 md:pt-0">
    <!-- Audio Settings -->
    <div class="card px-5 pb-4 sm:px-5">
        <div class="my-3 flex h-8n flex-col">
            <h2
                class="font-medium tracking-wide text-slate-700 line-clamp-1 dark:text-navy-100 lg:text-base pb-2"
            >
                Audio Settings
            </h2>
            <!-- Desktop description -->
            <p class="text-sm leading-6 hidden md:block">
                Configure your audio input and audio devices. Ensure both devices are configured to run at 48Khz Stereo.
            </p>
        </div>

        {#if isReady}
        <!-- Audio device selector component -->
        <AudioDeviceSelector
            layoutMode="horizontal"
            containerClass="hidden md:flex mb-4 -mx-2"
            deviceContainerClass="mt-5 flex-1 px-5"
            showLoadingText={false}
            {store}
            eventScope="#audio-settings-page"
        />

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

        <!-- Noise gate settings component -->
        <NoiseGateSettings
            toggleStyle="switch"
            knobsContainerClass="pt-5 pb-5 flex flex-row justify-evenly"
            showDescription={false}
            showDeepFilterNet={true}
            {store}
        />
        {/if}
    </div>
</div>
