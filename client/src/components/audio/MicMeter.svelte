<script lang="ts">
    import Ring from "$radial/components/Ring.svelte";

    interface Props {
        /**
         * The level to draw, 0 to 1, already on the meter's own scale.
         *
         * A display level rather than an RMS. The two callers no longer measure the same
         * thing — one reads a live session's published level, the other the raw amplitude of
         * a stream it started — so scaling belongs where the units are known. Scaling here as
         * well pinned the ring at full for anybody in a session.
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
        /**
         * `pane` fills a positioned visual half, which is how setup uses it. `card` is a
         * fixed-size ring beside its own reading, for a settings row.
         */
        layout?: "pane" | "card";
    }
    let { level, speaking, available = true, layout = "pane" }: Props = $props();

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
  The same ring the empty states use, so a microphone test and "nobody is here" are
  visibly the same object rather than two different widgets.

  No sources, and a still profile that spins: the mark alone is the reading. Coloured bars
  blooming beside it made two things move at once and left it ambiguous which was answering
  "can you hear me". `ringStill` keeps the ring's shape and lets `spin` sweep it round,
  which reads as one steady object turning rather than a second thing competing for
  attention.
-->
{#if layout === "pane"}
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
{:else}
    <div class="rad-mic-meter">
        <Ring
            mode={available ? "live" : "empty"}
            gain={gain}
            ringStill={true}
            spin={0.12}
            size={104}
        />
        <span class="rad-mic-meter__text">
            <span class="rad-label">{label}</span>
        </span>
    </div>
{/if}
