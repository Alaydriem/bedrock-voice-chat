<script lang="ts">
    import { onMount } from 'svelte';
    import { invoke } from "@tauri-apps/api/core";
    import { mount } from "svelte";
    import { info, error } from '@tauri-apps/plugin-log';
    import { Store } from '@tauri-apps/plugin-store';
    import PlatformDetector from '../../js/app/utils/PlatformDetector';
    import selectSvelte from '../forms/select.svelte';
    import type { AudioDevice } from '../../js/bindings/AudioDevice';

    // Props for customization
    interface Props {
        layoutMode?: "vertical" | "horizontal";
        containerClass?: string;
        deviceContainerClass?: string;
        showLoadingText?: boolean;
        store?: Store | null;
        eventScope?: string; // CSS selector scope for event listeners
    }

    let {
        layoutMode = "vertical",
        containerClass = "space-y-4",
        deviceContainerClass = "",
        showLoadingText = true,
        store = null,
        eventScope = "#audio-device-selector-component"
    }: Props = $props();

    let isMobile = $state(false);
    let inputDevices: AudioDevice[] = $state([]);
    let outputDevices: AudioDevice[] = $state([]);
    let platformDetector: PlatformDetector;
    let isLoading = $state(true);

    onMount(async () => {
        platformDetector = new PlatformDetector();
        isMobile = await platformDetector.checkMobile();

        if (!isMobile) {
            await loadDevices();
        }

        isLoading = false;
    });

    async function loadDevices() {
        try {
            const devices = await invoke<Record<string, AudioDevice[]>>("get_devices");

            let deviceTypes = [
                "WASAPI"
            ];

            if (await platformDetector.isWindows()) {
                deviceTypes.push("ASIO");
            }

            deviceTypes.forEach((type) => {
                if (devices[type]) {
                    devices[type].forEach((device: AudioDevice) => {
                        if (device.io === "InputDevice") {
                            inputDevices.push(device);
                        } else {
                            outputDevices.push(device);
                        }
                    });
                }
            });

            inputDevices.sort();
            outputDevices.sort();

            // Mount input device selector
            if (inputDevices.length > 0) {
                const currentInputDevice = await invoke<AudioDevice>("get_audio_device", { io: "InputDevice" });
                const inputContainer = document.getElementById("input-audio-device-container");
                if (inputContainer) {
                    mount(selectSvelte, {
                        target: inputContainer,
                        props: {
                            label: "Input Device",
                            id: "input-audio-device",
                            options: inputDevices,
                            defaultOption: currentInputDevice.display_name,
                        }
                    });
                }
            }

            // Mount output device selector
            if (outputDevices.length > 0) {
                const currentOutputDevice = await invoke<AudioDevice>("get_audio_device", { io: "OutputDevice" });
                const outputContainer = document.getElementById("output-audio-device-container");
                if (outputContainer) {
                    mount(selectSvelte, {
                        target: outputContainer,
                        props: {
                            label: "Output Device",
                            id: "output-audio-device",
                            options: outputDevices,
                            defaultOption: currentOutputDevice.display_name,
                        }
                    });
                }
            }

            // Add change event listeners
            const selects = document.querySelectorAll(`${eventScope} select`);
            selects.forEach((element) => {
                element.addEventListener("change", async (e) => {
                    const target = e.target as HTMLSelectElement;
                    const selectedOption = target.options[target.selectedIndex];
                    const deviceName = selectedOption.value;

                    let targetDevice: AudioDevice | undefined;
                    if (target.id === "input-audio-device") {
                        targetDevice = inputDevices.find(d => d.display_name === deviceName);
                    } else if (target.id === "output-audio-device") {
                        targetDevice = outputDevices.find(d => d.display_name === deviceName);
                    }

                    if (targetDevice) {
                        try {
                            await invoke("set_audio_device", { device: targetDevice });
                            info(`Audio device changed to ${targetDevice.display_name} for ${target.id}`);
                            await invoke("change_audio_device");
                        } catch (e) {
                            error(`Error changing audio device: ${e}`);
                        }
                    }
                });
            });
        } catch (e) {
            error(`Error loading audio devices: ${e}`);
        }
    }
</script>

<div id="audio-device-selector-component" class="audio-device-selector">
    {#if !isMobile}
        <div class:hidden={!isLoading}>
            {#if showLoadingText}
                <div class="flex flex-col items-center justify-center py-8 gap-4">
                    <div
                        class="spinner is-elastic size-12 animate-spin rounded-full border-[3px] border-secondary/30 border-r-secondary"
                    ></div>
                    <p class="text-slate-600 dark:text-navy-300">Getting your audio devices...</p>
                </div>
            {:else}
                <div class="flex justify-center">
                    <div
                        id="audio-device-select-spinner"
                        class="spinner size-7 animate-spin rounded-full border-[3px] border-warning/30 border-r-warning"
                    ></div>
                </div>
            {/if}
        </div>
        <div class={containerClass} class:hidden={isLoading}>
            <div id="input-audio-device-container" class={deviceContainerClass}></div>
            <div id="output-audio-device-container" class={deviceContainerClass}></div>
        </div>
    {:else}
        <p class="text-sm text-slate-600 dark:text-navy-300 italic">
            Device selection is not available on mobile platforms.
        </p>
    {/if}
</div>
