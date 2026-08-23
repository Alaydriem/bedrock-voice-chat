<script lang="ts">
    import Ring from "$radial/components/Ring.svelte";

    interface Props {
        /**
         * The level to draw, 0 to 1, already on the meter's own scale.
         *
         * A display level rather than an RMS: the caller measures the raw amplitude of the
         * metering stream it started, so scaling belongs where the units are known.
         */
        level: number;
        /** Whether audio is reaching the encoder right now. */
        speaking: boolean;
        /**
         * Capture started. False means the meter cannot move at all — a device that is
         * missing, or held exclusively by another application. Distinguishing that from a
         * quiet microphone is the whole point of the meter, and a flat mark says both.
         */
        available?: boolean;
    }
    let { level, speaking, available = true }: Props = $props();

    /**
     * The mark's amplitude is the reading. Reaching its full silhouette is what tells
     * someone the microphone they picked is the one BVC hears, so the scale is generous by
     * design — a normal speaking voice has to get there without being asked to shout.
     */
    let gain = $derived(available ? level : 0);

    /**
     * One line, saying what is happening rather than which component decided it.
     *
     * There was a second line under this in mono caps — "PASSING THE GATE", "BELOW THE GATE",
     * "SILENT" — which said the same thing twice and named a mechanism to do it. It also
     * described the wrong thing half the time: with the noise gate switched off there is no
     * gate to be below, and the reading was shown anyway.
     *
     * Removed rather than reworded. Two captions changing together is two chances to reflow,
     * and the mark beside it already answers "is it hearing me" without being read.
     */
    let label = $derived(
        !available
            ? "Cannot open that microphone"
            : speaking
              ? "We can hear you"
              : "Say something to test it",
    );
</script>

<!--
  Setup's microphone test, which is the one screen that opens a capture of its own — the audio
  settings pane draws the session's capture with a bare mark instead.

  The same ring the empty states use, so a microphone test and "nobody is here" are
  visibly the same object rather than two different widgets.

  No sources, and a still profile that spins: the mark alone is the reading. Coloured bars
  blooming beside it made two things move at once and left it ambiguous which was answering
  "can you hear me". `ringStill` keeps the ring's shape and lets `spin` sweep it round,
  which reads as one steady object turning rather than a second thing competing for
  attention.
-->
<div class="rad-visual">
    <Ring
        mode={available ? "live" : "empty"}
        gain={gain}
        ringStill={true}
        spin={0.12}
        class="rad-ring--fill"
    />
    <span class="rad-caption">
        <span class="rad-label">{label}</span>
    </span>
</div>
