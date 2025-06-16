<script lang="ts">
    import type { Store } from "@tauri-apps/plugin-store";
    import { invoke } from "@tauri-apps/api/core";
    import { onMount } from "svelte";
    import LittlePhatty from "../../../node_modules/webaudio-controls/knobs/LittlePhatty.png";
    
    export let id: string = "knob";
    export let src: string = "../../../../node_modules/webaudio-controls/knobs/LittlePhatty.png";
    export let value: number = 0;
    export let min: number = 0;
    export let max: number = 100;
    export let step: number = 1;
    export let label: string = "Knob";
    export let diameter: number = 64;
    export let sprites: number = 100;
    export let store: Store;

    onMount(() => {
        const knob = document.querySelector("webaudio-knob#" + id);
        let timeout;
        knob?.addEventListener("change", (e: Event) => {
            const target = e.target as HTMLInputElement;
            value = parseFloat(target.value);
            timeout = setTimeout(async () => {
                let noiseGateSettings = await store.get("noise_gate_settings") as { 
                    open_threshold: number,
                    close_threshold: number,
                    release_rate: number,
                    attack_rate: number,
                    hold_time: number
                };

                if (noiseGateSettings != null) {
                    switch (id) {
                        case "open_threshold":
                            noiseGateSettings.open_threshold = value;
                            break;
                        case "close_threshold":
                            noiseGateSettings.close_threshold = value;
                            break;
                        case "release_rate":
                            noiseGateSettings.release_rate = value;
                            break;
                        case "attack_rate":
                            noiseGateSettings.attack_rate = value;
                            break;
                        case "hold_time":
                            noiseGateSettings.hold_time = value;
                            break;
                    }

                    await store.set("noise_gate_settings", noiseGateSettings);
                    await store.save();

                    await invoke("update_stream_metadata", {
                        key: "noise_gate_settings",
                        value: JSON.stringify(noiseGateSettings),
                        device: "InputDevice"
                    });
                }
            }, 200);
        });
    });
</script>

<webaudio-knob
    class="inline-flex"
    id={id}
    valuetip=1
    src={LittlePhatty}
    value={value}
    min={min}
    max={max}
    step={step}
    label={label}
    diameter={diameter}
    sprites={sprites}
>{label}</webaudio-knob>