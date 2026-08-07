<script lang="ts">
    import AudioDeviceSelector from "../audio/AudioDeviceSelector.svelte";
    import MicMeter from "../audio/MicMeter.svelte";
    import { LevelScale } from "$radial/core/sources/LevelScale";
    import PlaybackTest from "../audio/PlaybackTest.svelte";
    import RadScreen from "../shell/RadScreen.svelte";
    import StepDots from "../shell/StepDots.svelte";

    interface Props {
        step: number;
        total: number;
        /** Post-gate RMS, 0 to 1, from the audio-input-level event. */
        inputLevel: number;
        gateOpen: boolean;
        /**
         * Capture started. False means the meter cannot move at all — a device that is
         * missing, or held exclusively by another application. Distinguishing that from a
         * quiet microphone is the whole point of this screen, and a flat meter says both.
         */
        available?: boolean;
        /** Play a chime through the selected output device. Resolves when it has finished. */
        ontestspeaker?: () => Promise<boolean>;
        oncontinue: () => void;
    }
    let {
        step,
        total,
        inputLevel,
        gateOpen,
        available = true,
        ontestspeaker,
        oncontinue,
    }: Props = $props();
</script>

<RadScreen label="Devices">
    {#snippet topbar()}
        <StepDots {step} {total} />
    {/snippet}

    <div class="rad-split">
        <div class="rad-visual-pane">
            <MicMeter level={LevelScale.fromRms(inputLevel)} speaking={gateOpen} {available} layout="pane" />
        </div>
        <div class="rad-content-pane rad-content-pane--top">
            <span class="rad-label rad-rise" style="--d: 50">03 &middot; Devices</span>
            <h2 class="rad-display rad-rise" style="--d: 120; margin-top: 12px; font-size: 2rem">
                Pick your microphone<br /><b>and where you listen.</b>
            </h2>
            <p class="rad-body rad-rise" style="--d: 210">
                Both can be changed later in settings. Talk for a moment and the mark beside this
                fills out as your voice passes the noise gate.
            </p>

            <div class="rad-rise" style="--d: 300; margin-top: 24px">
                <AudioDeviceSelector />
            </div>

            {#if ontestspeaker}
                <div class="rad-rise" style="--d: 360; margin-top: 20px">
                    <PlaybackTest ontest={ontestspeaker} />
                </div>
            {/if}
        </div>
    </div>

    {#snippet footbar()}
        <span class="rad-label">Changeable any time in settings</span>
        <button class="rad-btn rad-btn--lg rad-btn--primary" onclick={oncontinue}>
            Finish setup
        </button>
    {/snippet}
</RadScreen>
