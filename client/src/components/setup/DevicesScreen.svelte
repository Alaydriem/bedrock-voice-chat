<script lang="ts">
    import Ring from "$radial/components/Ring.svelte";
    import { LevelScale } from "$radial/core/sources/LevelScale";
    import AudioDeviceSelector from "../audio/AudioDeviceSelector.svelte";
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

    /**
     * The output half of the test. The microphone half reports itself continuously, but
     * nothing arrives to prove the speakers work — so without this the screen only ever
     * verifies half of what it is asking the user to choose.
     */
    let playing = $state(false);
    let outputFailed = $state(false);

    async function testSpeaker(): Promise<void> {
        if (playing || !ontestspeaker) return;
        playing = true;
        outputFailed = false;
        outputFailed = !(await ontestspeaker());
        playing = false;
    }

    /**
     * The mark's amplitude is the reading. Reaching its full silhouette is what tells
     * someone the microphone they picked is the one BVC hears, so the scale is generous by
     * design — a normal speaking voice has to get there without being asked to shout.
     */
    let level = $derived(available ? LevelScale.fromRms(inputLevel) : 0);

</script>

<RadScreen label="Devices">
    {#snippet topbar()}
        <StepDots {step} {total} />
    {/snippet}

    <div class="rad-split">
        <div class="rad-visual-pane">
            <!--
              The same ring the empty states use, so a microphone test and "nobody is
              here" are visibly the same object rather than two different widgets. It is
              centred and fills the pane; the level drives the mark inside it.

              No sources, and a still profile that spins: the mark alone is the reading.
              Coloured bars blooming beside it made two things move at once and left it
              ambiguous which was answering "can you hear me". `ringStill` keeps the ring's
              shape and lets `spin` sweep it round, which reads as one steady object turning
              rather than a second thing competing for attention.
            -->
            <div class="rad-visual">
                <Ring
                    mode={available ? "live" : "empty"}
                    gain={level}
                    ringStill={true}
                    spin={0.12}
                    class="rad-ring--fill"
                />
                <span class="rad-caption">
                    <span class="rad-label">
                        {#if !available}
                            Cannot open that microphone
                        {:else if gateOpen}
                            We can hear you
                        {:else}
                            Say something to test it
                        {/if}
                    </span>
                    <span class="rad-caption__value">
                        {#if !available}
                            NO INPUT
                        {:else if gateOpen}
                            PASSING THE GATE
                        {:else}
                            BELOW THE GATE
                        {/if}
                    </span>
                </span>
            </div>
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
                <AudioDeviceSelector layoutMode="vertical" showLoadingText={true} />
            </div>

            {#if ontestspeaker}
                <div class="rad-rise" style="--d: 360; margin-top: 20px">
                    <button
                        class="rad-btn rad-btn--lg"
                        onclick={testSpeaker}
                        disabled={playing}
                    >
                        {playing ? "Playing…" : "Test speaker"}
                    </button>
                    {#if outputFailed}
                        <div
                            class="rad-resolve rad-resolve--bad"
                            style="margin-top: 12px"
                            role="alert"
                        >
                            <span aria-hidden="true">&#10005;</span>
                            <span>Could not play through that device. Try another one.</span>
                        </div>
                    {:else}
                        <span class="rad-choice__note">
                            You should hear a short two-note chime.
                        </span>
                    {/if}
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
