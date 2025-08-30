import { info, error, warn } from '@tauri-apps/plugin-log';
import { invoke } from "@tauri-apps/api/core";
import { mount } from "svelte";
import { Store } from '@tauri-apps/plugin-store';

import '../../../../node_modules/webaudio-controls/webaudio-controls.js';

import selectSvelte from '../../../components/forms/select.svelte';
import Knob from '../../../components/forms/knob.svelte';

import type { AudioDevice } from '../../bindings/AudioDevice';
declare global {
  interface Window {
    App: any;
  }
}

export default class AudioSettings  {
    private store: Store | undefined;
    async initialize(osFamily: string) {
        this.store = await Store.load("store.json", { autoSave: false });

        // Don't load the devices options on mobile
        if (osFamily != "ios" && osFamily != "android") {
            document.getElementById("audio-settings-page")?.classList.remove("hidden");
            await invoke<Record<string, AudioDevice[]>>("get_devices")
            .then(async (devices) => {
                let inputDevices = Array<AudioDevice>();
                let outputDevices = Array<AudioDevice>();

                const deviceTypes = [
                    "WASAPI",
                    //"ASIO",
                ];

                deviceTypes.forEach((type) => {
                    if (devices[type]) {
                        devices[type].forEach((device: AudioDevice) => {
                            if (device.io == "InputDevice") {
                                inputDevices.push(device);
                            } else {
                                outputDevices.push(device);
                            }
                        });
                    }
                });

                const currentInputDevice = await invoke<AudioDevice>("get_audio_device", { io: "InputDevice" });
                mount(selectSvelte, {
                    target: document.getElementById("input-audio-device-container")!,
                    props: {
                        label: "Input Device",
                        id: "input-audio-device",
                        options: inputDevices,
                        defaultOption: currentInputDevice.display_name,
                    }
                });
                
                const currentOutputDevice = await invoke<AudioDevice>("get_audio_device", { io: "OutputDevice" });
                mount(selectSvelte, {
                    target: document.getElementById("output-audio-device-container")!,
                    props: {
                        label: "Output Device",
                        id: "output-audio-device",
                        options: outputDevices,
                        defaultOption: currentOutputDevice.display_name,
                    }
                });

                let elements = document.querySelectorAll("#audio-settings-page select");
                elements.forEach((element) => {
                    element.addEventListener("change", async (e) => {
                        const target = e.target as HTMLSelectElement;
                        const selectedOption = target.options[target.selectedIndex];
                        const deviceName = selectedOption.value;

                        let targetDevice: AudioDevice | undefined;
                        if (target.id === "input-audio-device") {
                            inputDevices.forEach((device) => {
                                if (device.display_name === deviceName) {
                                    targetDevice = device;
                                }
                            });
                        } else if (target.id === "output-audio-device") {
                            outputDevices.forEach((device) => {
                                if (device.display_name === deviceName) {
                                    targetDevice = device;
                                }
                            });
                        }

                        await invoke("set_audio_device", { device: targetDevice })
                        .then(async (result) => {
                            info(`Audio device changed to ${targetDevice?.display_name} for ${target.id}`);
                            await invoke("change_audio_device");
                        })
                        .catch((e) => {
                            error(`Error changing audio device: ${e}`);
                        })
                    });
                });

                document.getElementById("audio-device-select-spinner")?.classList.add("hidden");
            });
        }

        // Noise Gate Settings
        await this.store?.get<boolean>("use_noise_gate").then(async (useNoiseGate) => {
            const element = document.getElementById("noise-suppression-rs-toggle") as HTMLInputElement;
            // Check it to true
            if (useNoiseGate) {
                element.checked = true;
                document.getElementById("noise-gate-audio-controls")?.classList.remove("hidden");
            }

            // Enable the option
            element.disabled = false;

            // Add the event change listener`
            element.addEventListener("change", async (e) => {
                const target = e.target as HTMLInputElement;
                await this.store?.set("use_noise_gate", target.checked);
                await invoke("update_stream_metadata", {
                    key: "use_noise_gate",
                    value: target.checked ? "true" : "false",
                    device: "InputDevice",
                });

                document.getElementById("noise-gate-audio-controls")?.classList.toggle("hidden");

                await this.store?.save();
            });

            // Enable the settings with the default values
            let noiseGateSettings = await this.store?.get("noise_gate_settings") as { 
                open_threshold: number,
                close_threshold: number,
                release_rate: number,
                attack_rate: number,
                hold_time: number
            } | null;
            if (noiseGateSettings != null) {
                const container = document.getElementById("noise-gate-audio-controls");
                mount(Knob, {
                    target: container!,
                    props: {
                        label: "Open Threshold",
                        id: "open_threshold",
                        value: noiseGateSettings.open_threshold,
                        min: -96,
                        max: 0,
                        step: 1,
                        diameter: 64,
                        sprites: 100,
                        store: this.store!
                    }
                });

                mount(Knob, {
                    target: container!,
                    props: {
                        label: "Close Threshold",
                        id: "close_threshold",
                        value: noiseGateSettings.close_threshold,
                        min: -96,
                        max: 0,
                        step: 1,
                        diameter: 64,
                        sprites: 100,
                        store: this.store!
                    }
                });

                mount(Knob, {
                    target: container!,
                    props: {
                        label: "Attack Rate",
                        id: "attack_rate",
                        value: noiseGateSettings.attack_rate,
                        min: 0,
                        max: 250,
                        step: 1,
                        diameter: 64,
                        sprites: 100,
                        store: this.store!
                    }
                });

                mount(Knob, {
                    target: container!,
                    props: {
                        label: "Hold time",
                        id: "hold_time",
                        value: noiseGateSettings.hold_time,
                        min: 0,
                        max: 250,
                        step: 1,
                        diameter: 64,
                        sprites: 100,
                        store: this.store!
                    }
                });

                mount(Knob, {
                    target: container!,
                    props: {
                        label: "Release Rate",
                        id: "release_rate",
                        value: noiseGateSettings.release_rate,
                        min: 0,
                        max: 250,
                        step: 1,
                        diameter: 64,
                        sprites: 100,
                        store: this.store!
                    }
                });
            }
        });
    }
}