import { info, error, warn } from '@tauri-apps/plugin-log';
import { invoke } from "@tauri-apps/api/core";
import { mount } from "svelte";
import { Store } from '@tauri-apps/plugin-store';
import selectSvelte from '../../../components/forms/select.svelte';
import type { AudioDevice } from '../../bindings/AudioDevice';
declare global {
  interface Window {
    App: any;
  }
}

export default class AudioSettings  {
    private store: Store | undefined;
    async initialize() {
        this.store = await Store.load("store.json", { autoSave: false });

        document.getElementById("audio-settings-page")?.classList.remove("hidden");
        await invoke<Record<string, AudioDevice[]>>("get_devices")
        .then(async (devices) => {
            let inputDevices = Array<AudioDevice>();
            let outputDevices = Array<AudioDevice>();

            console.log("Devices: ", devices);
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

            document.getElementById("audio-device-select-spinner")?.classList.add("hidden");
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
                        console.log("input device changed {deviceName: ", deviceName);
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

                    console.log("{targetDevice: ", targetDevice);
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
        });

        // Noise Gate Settings
        await this.store?.get<boolean>("use_noise_gate").then((useNoiseGate) => {
            const element = document.getElementById("noise-suppression-rs-toggle") as HTMLInputElement;
            // Check it to true
            if (useNoiseGate) {
                element.checked = true;
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
                }).then((result) => {
                    console.log(result);
                }).catch((e) => {
                    error(`Error updating stream metadata: ${e}`);
                });
                await this.store?.save();
            });

            // Enable the settings with the default values
        });
    }
}