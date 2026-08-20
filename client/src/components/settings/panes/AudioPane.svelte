<script lang="ts">
  import { I18n } from "$lib/i18n";
    import { getContext, onDestroy, onMount } from "svelte";
    import { AUDIO_SETTINGS_KEY } from "../../../js/app/shell/AudioSettingsContext";
    import Segmented from "$radial/components/Segmented.svelte";
    import SettingRow from "$radial/components/SettingRow.svelte";
    import Toggle from "$radial/components/Toggle.svelte";
    import StatusChip from "$radial/components/StatusChip.svelte";
    import AudioDeviceSelector from "../../audio/AudioDeviceSelector.svelte";
    import LevelMeter from "$radial/components/LevelMeter.svelte";
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

    /**
     * The layout's instance when there is one, so the jukebox chip in the header and this pane
     * cannot disagree about the same setting. A fresh one otherwise, so the pane still works
     * mounted on its own.
     */
    const shared = getContext<AudioSettingsManager | undefined>(AUDIO_SETTINGS_KEY);
    const audio = shared ?? new AudioSettingsManager();
    const probe = new InputLevelProbe();
    const speaker = new SpeakerTest();

    let voiceMode = $state<VoiceMode>("openMic");
    let voiceModeError = $state("");
    let panning = $state(100);
    let jukeboxGain = $state(100);
    let jukeboxMuted = $state(false);
    let muteCues = $state(true);

    const unsubs: Array<() => void> = [];

    onMount(() => {
        unsubs.push(audio.voiceMode.subscribe((v) => (voiceMode = v)));
        unsubs.push(audio.voiceModeError.subscribe((v) => (voiceModeError = v)));
        unsubs.push(audio.panningIntensity.subscribe((v) => (panning = v)));
        unsubs.push(audio.jukeboxGain.subscribe((v) => (jukeboxGain = v)));
        unsubs.push(audio.jukeboxMuted.subscribe((v) => (jukeboxMuted = v)));
        unsubs.push(audio.muteCues.subscribe((v) => (muteCues = v)));
        // The layout initialises the shared instance; only a standalone one needs it here.
        if (!shared) void audio.initialize();
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
            <!-- The mark alone, centred. The row's own label and note already say what this is
                 and what to do with it, so a caption under it said the same thing twice. -->
            <div class="rad-mic-meter">
                <LevelMeter source={probe.source} cell={3} />
            </div>
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

        <SettingRow
            label={I18n.t("Mute and deafen sounds")}
            note={I18n.t("A short tone when you mute or deafen, and a rising one when you turn either back on. It falls for off and rises for on, so you can tell without looking at the window.")}
        >
            {#snippet control()}
                <Toggle
                    checked={muteCues}
                    label={I18n.t("Mute and deafen sounds")}
                    onchange={(next) => void audio.handleMuteCuesChange(next)}
                />
            {/snippet}
        </SettingRow>
    </div>

    <div class="rad-card">
        <div class="rad-card__head">{I18n.t("Jukeboxes")}</div>

        <SettingRow
            label={I18n.t("Mute jukeboxes")}
            note={I18n.t("Music from jukeboxes in the world. Voices are not affected.")}
        >
            {#snippet control()}
                <Toggle
                    checked={jukeboxMuted}
                    label={I18n.t("Mute jukeboxes")}
                    onchange={(next) => void audio.handleJukeboxMutedChange(next)}
                />
            {/snippet}
        </SettingRow>

        <SettingRow
            label={I18n.t("Jukebox volume")}
            note={I18n.t("How loud jukebox music plays before distance is applied. Every jukebox keeps its own position and its own falloff, so this scales what distance already decided.")}
            stack
        >
            <div class="rad-knob__head">
                <span class="rad-knob__label">
                    {jukeboxMuted ? I18n.t("Muted") : I18n.t("Playing")}
                </span>
                <span class="rad-knob__value">{jukeboxGain}%</span>
            </div>
            <input
                class="rad-range"
                type="range"
                min="0"
                max="150"
                step="5"
                value={jukeboxGain}
                disabled={jukeboxMuted}
                style="width: 100%"
                aria-label={I18n.t("Jukebox volume")}
                aria-valuetext="{jukeboxGain}%"
                oninput={(e) =>
                    void audio.handleJukeboxGainChange(
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
