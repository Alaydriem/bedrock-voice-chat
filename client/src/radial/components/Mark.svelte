<script lang="ts">
  import { onDestroy } from "svelte";
  import { MarkBinding } from "$radial/bindings/MarkBinding";

  interface Props {
    /** Block size in CSS px. Overridden by `--rad-mark-cell` when CSS sets it. */
    cell?: number;
    /** Space between blocks. Defaults to 30% of the cell. */
    gap?: number;
    /** A hex colour, or `rainbow` for the spectrum. */
    color?: string | "rainbow";
    /** Violet behind the blocks. Off for small meters, where it reads as noise. */
    mortar?: boolean;
    /** Amplitude, 0 collapses to the mid row and 1 is full height. */
    gain?: number;
    /** Hold the amplitude still rather than dancing. */
    still?: boolean;
    class?: string;
  }

  let {
    cell = 6,
    gap,
    color = "rainbow",
    mortar = true,
    gain = 1,
    still = false,
    class: className = "",
  }: Props = $props();

  let canvas: HTMLCanvasElement;
  let binding: MarkBinding | null = null;

  $effect(() => {
    binding = new MarkBinding(canvas, { cell, gap, color, mortar, gain, still, bleed: mortar });
    return () => {
      binding?.destroy();
      binding = null;
    };
  });

  // Amplitude and colour change per frame in normal use, so they are pushed onto the
  // live binding rather than remounting it.
  $effect(() => {
    if (binding) binding.gain = gain;
  });
  $effect(() => {
    if (binding) binding.color = color;
  });

  onDestroy(() => binding?.destroy());
</script>

<canvas bind:this={canvas} class={className} data-rad-mark></canvas>
