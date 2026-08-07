<script lang="ts">
    import { invoke } from "@tauri-apps/api/core";
    import { Store } from "@tauri-apps/plugin-store";
    import { onMount } from "svelte";
    import Icon from "$radial/components/Icon.svelte";
    import SettingRow from "$radial/components/SettingRow.svelte";
    import Toggle from "$radial/components/Toggle.svelte";
    import Analytics from "../../js/app/analytics";
    import {
        NoiseGateModel,
        type NoiseGateSettings,
    } from "../../js/app/settings/NoiseGateModel";

    let store = $state<Store | null>(null);
    let enabled = $state(false);
    let settings = $state<NoiseGateSettings>({ ...NoiseGateModel.DEFAULTS });

    onMount(async () => {
        try {
            const loaded = await Store.load("store.json", { autoSave: false, defaults: {} });
            store = loaded;
            enabled = (await loaded.get<boolean>("use_noise_gate")) ?? false;
            settings = NoiseGateModel.hydrate(
                await loaded.get<Partial<NoiseGateSettings>>("noise_gate_settings"),
            );
        } catch {
            // Falls back to the defaults already in `settings`.
        }
    });

    async function persistEnabled(next: boolean): Promise<void> {
        enabled = next;
        await store?.set("use_noise_gate", next);
        await store?.save();
        await invoke("update_stream_metadata", {
            key: "use_noise_gate",
            value: next ? "true" : "false",
            device: "InputDevice",
        }).catch(() => {});
        Analytics.track("NoiseGateToggled", { enabled: next ? 1 : 0 });
    }

    /** Called on release, not per frame: this reaches into the live audio path. */
    async function persistSettings(next: NoiseGateSettings): Promise<void> {
        settings = next;
        await store?.set("noise_gate_settings", next);
        await store?.save();
        await invoke("update_stream_metadata", {
            key: "noise_gate_settings",
            value: JSON.stringify(next),
            device: "InputDevice",
        }).catch(() => {});
    }

    /** Updates the readout only. */
    function preview(id: keyof NoiseGateSettings, value: number): void {
        settings = NoiseGateModel.apply(settings, id, value);
    }
</script>

<SettingRow
    label="Cut my mic when I am not speaking"
    note="Runs on this machine. Anything it filters out is never encoded or sent."
>
    {#snippet control()}
        <Toggle
            checked={enabled}
            label="Noise gate"
            onchange={(next) => void persistEnabled(next)}
        />
    {/snippet}
</SettingRow>

{#if enabled}
    <div class="rad-knobs">
        {#each NoiseGateModel.KNOBS as knob (knob.id)}
            <div class="rad-knob">
                <span class="rad-knob__head">
                    <span class="rad-knob__label">{knob.label}</span>
                    <span class="rad-knob__value">
                        {NoiseGateModel.format(knob, settings[knob.id])}
                    </span>
                </span>
                <input
                    class="rad-range"
                    type="range"
                    min={knob.min}
                    max={knob.max}
                    step="1"
                    value={settings[knob.id]}
                    aria-label={knob.label}
                    oninput={(e) => preview(knob.id, Number((e.target as HTMLInputElement).value))}
                    onchange={() => void persistSettings(settings)}
                />
                <span class="rad-knob__note">{knob.note}</span>
            </div>
        {/each}
    </div>

    <SettingRow
        label="Back to the defaults"
        note=""
    >
        {#snippet control()}
            <button
                class="rad-btn"
                onclick={() => void persistSettings({ ...NoiseGateModel.DEFAULTS })}
            >
                <Icon name="reset" /> Reset
            </button>
        {/snippet}
    </SettingRow>
{/if}
