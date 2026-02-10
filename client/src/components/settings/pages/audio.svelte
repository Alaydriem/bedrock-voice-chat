<script lang="ts">
    import { onMount } from "svelte";
    import { invoke } from "@tauri-apps/api/core";
    import { Store } from '@tauri-apps/plugin-store';
    import AudioDeviceSelector from '../../audio/AudioDeviceSelector.svelte';
    import NoiseGateSettings from '../../audio/NoiseGateSettings.svelte';
    import PlatformDetector from "../../../js/app/utils/PlatformDetector.ts";
    import type { KeybindConfig } from "../../../js/bindings/KeybindConfig.ts";
    import type { VoiceMode } from "../../../js/bindings/VoiceMode.ts";

    let store: Store | undefined = $state(undefined);
    let isReady = $state(false);
    let isMobile = $state(false);
    let voiceMode: VoiceMode = $state("openMic");

    async function handleVoiceModeChange(mode: VoiceMode) {
        voiceMode = mode;
        if (!store) return;

        const saved = await store.get<KeybindConfig>("keybinds");
        const config: KeybindConfig = {
            toggleMute: saved?.toggleMute ?? "BracketLeft",
            toggleDeafen: saved?.toggleDeafen ?? "BracketRight",
            toggleRecording: saved?.toggleRecording ?? "Backslash",
            pushToTalk: saved?.pushToTalk ?? "Backquote",
            voiceMode: mode,
        };
        await store.set("keybinds", config);
        await store.save();
        await invoke('start_keybind_listener', { config });
    }

    onMount(async () => {
        store = await Store.load("store.json", {
            autoSave: false,
            defaults: {}
        });

        const platformDetector = new PlatformDetector();
        isMobile = await platformDetector.checkMobile();

        // Load voice mode from keybinds config
        const saved = await store.get<KeybindConfig>("keybinds");
        if (saved?.voiceMode) {
            voiceMode = saved.voiceMode;
        }

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

        {#if !isMobile}
        <div class="my-4 h-px bg-slate-200 dark:bg-navy-500"></div>

        <div class="my-3 flex h-8n flex-col">
            <h2
                class="font-medium tracking-wide text-slate-700 line-clamp-1 dark:text-navy-100 lg:text-base pb-2"
            >
                Voice Mode
            </h2>
            <p class="text-sm leading-6">
                Choose how your microphone is activated.
            </p>
        </div>

        <div class="flex flex-col space-y-3 mt-2">
            <label class="flex items-center space-x-3 cursor-pointer py-2 px-3 rounded-lg hover:bg-slate-50 dark:hover:bg-navy-600 transition-colors">
                <input
                    type="radio"
                    name="voiceMode"
                    value="openMic"
                    checked={voiceMode === "openMic"}
                    onchange={() => handleVoiceModeChange("openMic")}
                    class="form-radio is-basic h-5 w-5 rounded-full border-slate-300/70 bg-slate-100 checked:border-primary checked:bg-primary hover:border-primary focus:border-primary dark:border-navy-400 dark:bg-navy-700 dark:checked:border-accent dark:checked:bg-accent dark:hover:border-accent dark:focus:border-accent"
                />
                <div>
                    <span class="text-sm font-medium text-slate-700 dark:text-navy-100">Open Mic</span>
                    <p class="text-xs text-slate-500 dark:text-navy-300">Microphone is always active. Use toggle mute to silence.</p>
                </div>
            </label>
            <label class="flex items-center space-x-3 cursor-pointer py-2 px-3 rounded-lg hover:bg-slate-50 dark:hover:bg-navy-600 transition-colors">
                <input
                    type="radio"
                    name="voiceMode"
                    value="pushToTalk"
                    checked={voiceMode === "pushToTalk"}
                    onchange={() => handleVoiceModeChange("pushToTalk")}
                    class="form-radio is-basic h-5 w-5 rounded-full border-slate-300/70 bg-slate-100 checked:border-primary checked:bg-primary hover:border-primary focus:border-primary dark:border-navy-400 dark:bg-navy-700 dark:checked:border-accent dark:checked:bg-accent dark:hover:border-accent dark:focus:border-accent"
                />
                <div>
                    <span class="text-sm font-medium text-slate-700 dark:text-navy-100">Push to Talk</span>
                    <p class="text-xs text-slate-500 dark:text-navy-300">Hold a key to unmute. Configure the key in Keybinds settings.</p>
                </div>
            </label>
        </div>
        {/if}
        {/if}
    </div>
</div>
