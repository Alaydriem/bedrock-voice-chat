<script lang="ts">
    import { onMount, mount, tick } from 'svelte';
    import { invoke } from "@tauri-apps/api/core";
    import { Store } from '@tauri-apps/plugin-store';
    import '../../../node_modules/webaudio-controls/webaudio-controls.js';
    import Knob from '../forms/knob.svelte';

    // Props for customization
    interface Props {
        toggleStyle?: "switch" | "checkbox";
        knobsContainerClass?: string;
        showDescription?: boolean;
        showDeepFilterNet?: boolean;
        store?: Store | null;
    }

    let {
        toggleStyle = "checkbox",
        knobsContainerClass = "grid grid-cols-2 md:grid-cols-5 gap-4 mt-6",
        showDescription = true,
        showDeepFilterNet = false,
        store: storeProps = null
    }: Props = $props();

    let store: Store;
    let isLoading = $state(true);
    let noiseGateEnabled = $state(false);

    interface NoiseGateSettingsType {
        open_threshold: number;
        close_threshold: number;
        release_rate: number;
        attack_rate: number;
        hold_time: number;
    }

    onMount(async () => {
        try {
            // Use passed store or load new one
            if (storeProps) {
                store = storeProps;
            } else {
                store = await Store.load("store.json", {
                    autoSave: false,
                    defaults: {}
                });
            }

            const useNoiseGate = await store.get<boolean>("use_noise_gate");
            noiseGateEnabled = useNoiseGate || false;

            console.log("NoiseGate - useNoiseGate from store:", useNoiseGate);
            console.log("NoiseGate - noiseGateEnabled:", noiseGateEnabled);

            // Set isLoading to false first so the DOM renders
            isLoading = false;

            if (noiseGateEnabled) {
                // Wait for Svelte to update the DOM (render the container)
                await tick();
                await mountKnobs();
            }
        } catch (error) {
            console.error("NoiseGateSettings initialization error:", error);
            isLoading = false;
        }
    });

    async function handleToggle() {
        noiseGateEnabled = !noiseGateEnabled;
        await store.set("use_noise_gate", noiseGateEnabled);
        await invoke("update_stream_metadata", {
            key: "use_noise_gate",
            value: noiseGateEnabled ? "true" : "false",
            device: "InputDevice",
        });
        await store.save();

        if (noiseGateEnabled) {
            // Wait for Svelte to update the DOM (unhide the container)
            await tick();
            await mountKnobs();
        }
    }

    async function mountKnobs() {
        console.log("mountKnobs called");
        // Wait for DOM to update and unhide the container
        await new Promise(resolve => setTimeout(resolve, 100));

        let noiseGateSettings = await store.get("noise_gate_settings") as NoiseGateSettingsType | null;
        console.log("NoiseGate settings from store:", noiseGateSettings);

        if (!noiseGateSettings) {
            noiseGateSettings = {
                open_threshold: -40,
                close_threshold: -50,
                release_rate: 100,
                attack_rate: 10,
                hold_time: 50
            };
            console.log("Using default noise gate settings");
        }

        const container = document.getElementById("noise-gate-audio-controls");
        console.log("Container found:", container);
        if (!container) {
            console.error("noise-gate-audio-controls container not found!");
            return;
        }

        // Clear container first
        container.innerHTML = '';
        console.log("Container cleared, mounting knobs...");

        try {
            mount(Knob, {
                target: container,
                props: {
                    label: "Open Threshold",
                    id: "open_threshold",
                    value: noiseGateSettings.open_threshold,
                    min: -96,
                    max: 0,
                    step: 1,
                    diameter: 64,
                    sprites: 100,
                    store: store
                }
            });
            console.log("Mounted: Open Threshold");

            mount(Knob, {
                target: container,
                props: {
                    label: "Close Threshold",
                    id: "close_threshold",
                    value: noiseGateSettings.close_threshold,
                    min: -96,
                    max: 0,
                    step: 1,
                    diameter: 64,
                    sprites: 100,
                    store: store
                }
            });
            console.log("Mounted: Close Threshold");

            mount(Knob, {
                target: container,
                props: {
                    label: "Attack Rate",
                    id: "attack_rate",
                    value: noiseGateSettings.attack_rate,
                    min: 0,
                    max: 250,
                    step: 1,
                    diameter: 64,
                    sprites: 100,
                    store: store
                }
            });
            console.log("Mounted: Attack Rate");

            mount(Knob, {
                target: container,
                props: {
                    label: "Hold time",
                    id: "hold_time",
                    value: noiseGateSettings.hold_time,
                    min: 0,
                    max: 250,
                    step: 1,
                    diameter: 64,
                    sprites: 100,
                    store: store
                }
            });
            console.log("Mounted: Hold time");

            mount(Knob, {
                target: container,
                props: {
                    label: "Release Rate",
                    id: "release_rate",
                    value: noiseGateSettings.release_rate,
                    min: 0,
                    max: 250,
                    step: 1,
                    diameter: 64,
                    sprites: 100,
                    store: store
                }
            });
            console.log("Mounted: Release Rate");
            console.log("All knobs mounted successfully!");
        } catch (error) {
            console.error("Error mounting knobs:", error);
        }
    }
</script>

<div class="noise-gate-settings">
    {#if isLoading}
    <div class="flex justify-center py-4">
        <div class="spinner size-7 animate-spin rounded-full border-[3px] border-warning/30 border-r-warning"></div>
    </div>
    {:else}
    <div class="flex mb-4 -mx-2 flex-col">
        <label class="inline-flex items-center space-x-2 pb-2">
            <input
                id="noise-suppression-rs-toggle"
                type="checkbox"
                bind:checked={noiseGateEnabled}
                on:change={handleToggle}
                class={toggleStyle === "switch"
                    ? "form-switch h-5 w-10 rounded-full bg-slate-300 before:rounded-full before:bg-slate-50 checked:bg-primary checked:before:bg-white dark:bg-navy-900 dark:before:bg-navy-300 dark:checked:bg-accent dark:checked:before:bg-white"
                    : "form-checkbox h-5 w-5 rounded border-slate-300 bg-transparent checked:border-primary checked:bg-primary hover:border-primary focus:border-primary dark:border-navy-400 dark:checked:border-accent dark:checked:bg-accent dark:hover:border-accent dark:focus:border-accent"}
            />
            <span x-tooltip.light="'A standard noise gate modeled after OBS\' Noise Gate Filter. Effective, but requires manual tuning for your environment.'">
                Noise Gate RS
            </span>
        </label>
        {#if showDescription && toggleStyle === "checkbox"}
        <p class="text-xs text-slate-500 dark:text-navy-400 mt-1 ml-8">
            Reduces background noise when you're not speaking
        </p>
        {/if}

        <div id="noise-gate-audio-controls" class={knobsContainerClass} class:hidden={!noiseGateEnabled}></div>

        {#if showDeepFilterNet}
        <label class="inline-flex items-center space-x-2 pb-2 pt-2">
            <input
                disabled
                type="checkbox"
                class={toggleStyle === "switch"
                    ? "form-switch h-5 w-10 rounded-full bg-slate-300 before:rounded-full before:bg-slate-50 checked:bg-primary checked:before:bg-white dark:bg-navy-900 dark:before:bg-navy-300 dark:checked:bg-accent dark:checked:before:bg-white"
                    : "form-checkbox h-5 w-5 rounded border-slate-300 bg-transparent checked:border-primary checked:bg-primary hover:border-primary focus:border-primary dark:border-navy-400 dark:checked:border-accent dark:checked:bg-accent dark:hover:border-accent dark:focus:border-accent"}
            />
            <span x-tooltip.light="'Experimental. A more advanced filtering neural network.'">
                Deep Filter Net
            </span>
        </label>
        {/if}
    </div>
    {/if}
</div>
