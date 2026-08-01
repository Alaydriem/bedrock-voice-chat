<script lang="ts">
  import { onDestroy } from "svelte";
  import { IntroSequence } from "$radial/core/intro/IntroSequence";

  interface Props {
    /** While true the mark dances. Flip to false and it collapses to its flat row. */
    loading?: boolean;
    /** Play the full boot sequence before settling into the dance. */
    withIntro?: boolean;
    size?: number;
    /** Null keeps the alpha channel, so the loader sits on whatever is behind it. */
    background?: string | null;
    /** Fires once the collapse has landed. */
    onfinished?: () => void;
  }

  let {
    loading = true,
    withIntro = false,
    size = 330,
    background = null,
    onfinished,
  }: Props = $props();

  let canvas: HTMLCanvasElement;
  let sequence: IntroSequence | null = null;
  let collapsing = false;

  $effect(() => {
    sequence = new IntroSequence(canvas, { width: size, height: size, background });
    sequence.startLoading(withIntro);
    return () => {
      sequence?.stop();
      sequence = null;
    };
  });

  // The collapse is the completion signal, so the caller can route on a finished
  // animation rather than on a timeout that may or may not have been long enough.
  $effect(() => {
    if (loading || collapsing || !sequence) return;
    collapsing = true;
    void sequence.finishCollapse().then(() => onfinished?.());
  });

  onDestroy(() => sequence?.stop());
</script>

<canvas bind:this={canvas}></canvas>
