<script lang="ts">
    import { onDestroy, onMount } from "svelte";
    import Segmented from "$radial/components/Segmented.svelte";
    import SettingRow from "$radial/components/SettingRow.svelte";
    import StatusChip from "$radial/components/StatusChip.svelte";
    import AudioDeviceSelector from "../../audio/AudioDeviceSelector.svelte";
    import NoiseGate from "../NoiseGate.svelte";
    import { AudioSettingsManager } from "../../../js/app/managers/settings/AudioSettingsManager";
    import type { VoiceMode } from "../../../js/bindings/VoiceMode";

    interface Props {
        mobile?: boolean;
    }
    let { mobile = false }: Props = $props();

    const audio = new AudioSettingsManager();

    let voiceMode = $state<VoiceMode>("openMic");
    let panning = $state(100);

    const unsubs: Array<() => void> = [];

    onMount(() => {
        unsubs.push(audio.voiceMode.subscribe((v) => (voiceMode = v)));
        unsubs.push(audio.panningIntensity.subscribe((v) => (panning = v)));
        void audio.initialize();
    });

    onDestroy(() => {
        for (const off of unsubs) off();
    });
</script>

<div class="rad-section">
    <div class="rad-card">
        <div class="rad-card__head">Devices</div>
        {#if mobile}
            <!-- Android and iOS route audio themselves: the OS follows the headset, and an
                 app-level picker there is a control that either lies or fights the system. -->
            <SettingRow
                label="Chosen by the system"
                note="Your phone routes voice to whatever you last connected. Plug in a headset and it follows — there is nothing to pick here."
            >
                {#snippet control()}
                    <StatusChip severity="muted">System default</StatusChip>
                {/snippet}
            </SettingRow>
        {:else}
            <AudioDeviceSelector />
        {/if}
    </div>

    <div class="rad-card">
        <div class="rad-card__head">Voice</div>

        <SettingRow label="Voice mode">
            {#snippet control()}
                <Segmented
                    options={[
                        { value: "openMic", label: "Voice activated" },
                        { value: "pushToTalk", label: "Push-to-talk" },
                    ]}
                    value={voiceMode}
                    onchange={(next) => void audio.handleVoiceModeChange(next as VoiceMode)}
                />
            {/snippet}
        </SettingRow>

        <SettingRow
            label="Spatial panning"
            note="How hard voices are pushed left and right by where their speaker is standing. At 0% everyone is centred; distance still governs volume either way."
            stack
        >
            <div class="rad-knob__head">
                <span class="rad-knob__label">
                    {panning === 0 ? "Everyone centred" : "Left and right"}
                </span>
                <span class="rad-knob__value">{panning}%</span>
            </div>
            <input
                class="rad-range"
                type="range"
                min="0"
                max="100"
                value={panning}
                style="width: 100%"
                aria-label="Spatial panning"
                aria-valuetext="{panning}%"
                oninput={(e) =>
                    void audio.handlePanningIntensityChange(
                        Number((e.target as HTMLInputElement).value),
                    )}
            />
        </SettingRow>
    </div>

    <div class="rad-card">
        <div class="rad-card__head">Noise gate</div>
        <NoiseGate />
    </div>
</div>
