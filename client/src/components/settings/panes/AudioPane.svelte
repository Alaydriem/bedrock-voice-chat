<script lang="ts">
  import { I18n } from "$lib/i18n";
    import { onDestroy, onMount } from "svelte";
    import Segmented from "$radial/components/Segmented.svelte";
    import SettingRow from "$radial/components/SettingRow.svelte";
    import StatusChip from "$radial/components/StatusChip.svelte";
    import AudioDeviceSelector from "../../audio/AudioDeviceSelector.svelte";
    import MicMeter from "../../audio/MicMeter.svelte";
    import PlaybackTest from "../../audio/PlaybackTest.svelte";
    import NoiseGate from "../NoiseGate.svelte";
    import { AudioSettingsManager } from "../../../js/app/managers/settings/AudioSettingsManager";
    import { InputLevelProbe } from "../../../js/app/settings/InputLevelProbe";
    import SpeakerTest from "../../../js/app/setup/SpeakerTest";
    import type { VoiceMode } from "../../../js/bindings/VoiceMode";

    interface Props {
        mobile?: boolean;
    }
    let { mobile = false }: Props = $props();

    const audio = new AudioSettingsManager();
    const probe = new InputLevelProbe();
    const speaker = new SpeakerTest();

    let voiceMode = $state<VoiceMode>("openMic");
    let voiceModeError = $state("");
    let panning = $state(100);
    let inputLevel = $state(0);
    let gateOpen = $state(false);
    let meterAvailable = $state(true);

    const unsubs: Array<() => void> = [];

    onMount(() => {
        unsubs.push(audio.voiceMode.subscribe((v) => (voiceMode = v)));
        unsubs.push(audio.voiceModeError.subscribe((v) => (voiceModeError = v)));
        unsubs.push(audio.panningIntensity.subscribe((v) => (panning = v)));
        unsubs.push(probe.rms.subscribe((v) => (inputLevel = v)));
        unsubs.push(probe.gateOpen.subscribe((v) => (gateOpen = v)));
        unsubs.push(probe.available.subscribe((v) => (meterAvailable = v)));
        void audio.initialize();
        void probe.start();
    });

    onDestroy(() => {
        for (const off of unsubs) off();
        void probe.stop();
    });
</script>

<div class="rad-section">
    <div class="rad-card">
        <div class="rad-card__head">{I18n.t("Devices")}</div>
        {#if mobile}
            <!-- Android and iOS route audio themselves: the OS follows the headset, and an
                 app-level picker there is a control that either lies or fights the system. -->
            <SettingRow
                label={I18n.t("Chosen by the system")}
                note={I18n.t("Your phone routes voice to whatever you last connected. Plug in a headset and it follows — there is nothing to pick here.")}
            >
                {#snippet control()}
                    <StatusChip severity="muted">{I18n.t("System default")}</StatusChip>
                {/snippet}
            </SettingRow>
        {:else}
            <AudioDeviceSelector />
        {/if}
    </div>

    <!-- Directly under the picker, because it answers the question the picker raises. On a
         phone there is nothing to pick and the test is the entire value of the card: the OS
         chose the route and this is the only way to find out what it chose. -->
    <div class="rad-card">
        <div class="rad-card__head">{I18n.t("Test your devices")}</div>

        <SettingRow
            label={I18n.t("Test my microphone")}
            note={I18n.t("Talk for a moment. The mark fills out as it hears you.")}
            stack
        >
            <MicMeter level={inputLevel} speaking={gateOpen} available={meterAvailable} layout="card" />
        </SettingRow>

        <SettingRow
            label={I18n.t("Test playback")}
            note={I18n.t("Plays a chime through the device you listen on, not through whatever the browser would pick.")}
        >
            {#snippet control()}
                <PlaybackTest ontest={() => speaker.play()} />
            {/snippet}
        </SettingRow>
    </div>

    <div class="rad-card">
        <div class="rad-card__head">{I18n.t("Voice")}</div>

        <SettingRow label={I18n.t("Voice mode")}>
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

        {#if voiceModeError}
            <div class="rad-callout rad-callout--warn">
                <span>
                    <b>{I18n.t("The voice mode did not change.")}</b>
                    {voiceModeError}
                </span>
            </div>
        {/if}

        <SettingRow
            label={I18n.t("Spatial panning")}
            note={I18n.t("How hard voices are pushed left and right by where their speaker is standing. At 0% everyone is centred; distance still governs volume either way.")}
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
                aria-label={I18n.t("Spatial panning")}
                aria-valuetext="{panning}%"
                oninput={(e) =>
                    void audio.handlePanningIntensityChange(
                        Number((e.target as HTMLInputElement).value),
                    )}
            />
        </SettingRow>
    </div>

    <div class="rad-card">
        <div class="rad-card__head">{I18n.t("Noise gate")}</div>
        <NoiseGate />
    </div>
</div>
