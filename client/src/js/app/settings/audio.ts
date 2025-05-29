
import { Store } from '@tauri-apps/plugin-store';
import { info, error, warn } from '@tauri-apps/plugin-log';
import { invoke } from "@tauri-apps/api/core";
import { onMount, mount } from "svelte";

import selectSvelte from '../../../components/forms/select.svelte';
import type { AudioDevice } from '../../bindings/AudioDevice';
declare global {
  interface Window {
    App: any;
  }
}

export default class AudioSettings  {
    async initialize() {
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

            console.log(currentInputDevice)
            
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

            document.getElementById("audio-settings-page")?.classList.remove("hidden");

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
        });
    }
}