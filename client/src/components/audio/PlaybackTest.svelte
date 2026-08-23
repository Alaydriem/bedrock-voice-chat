<script lang="ts">
  import { I18n } from "$lib/i18n";
    interface Props {
        /** Play a chime through the selected output device. Resolves when it has finished. */
        ontest: () => Promise<boolean>;
    }
    let { ontest }: Props = $props();

    /**
     * The output half of a device test. The microphone half reports itself continuously, but
     * nothing arrives to prove the speakers work — so without this a device screen only ever
     * verifies half of what it is asking somebody to choose.
     */
    let playing = $state(false);
    let failed = $state(false);

    async function play(): Promise<void> {
        if (playing) return;
        playing = true;
        failed = false;
        failed = !(await ontest());
        playing = false;
    }
</script>

<!--
  The button, and nothing beside it unless something went wrong.

  There was a standing note here saying a chime would play, which the row's own note already
  says. Two lines describing one button is one of them too many, and it changed places with the
  failure message — so the row's height moved whenever the test was run.
-->
<button class="rad-btn rad-btn--lg" onclick={play} disabled={playing}>
    {playing ? "Playing…" : "Test playback"}
</button>

{#if failed}
    <div class="rad-resolve rad-resolve--bad" style="margin-top: 12px" role="alert">
        <span aria-hidden="true">&#10005;</span>
        <span>{I18n.t("Could not play through that device. Try another one.")}</span>
    </div>
{/if}
