<script lang="ts">
  import { onDestroy } from "svelte";
  import { LevelMeterBinding } from "$radial/bindings/LevelMeterBinding";
  import type { LevelSource } from "$radial/core/sources/LevelSource";

  interface Props {
    /** Where the level comes from. Without one, drive `level` instead. */
    source?: LevelSource;
    /** Direct level, 0 to 1. Ignored when a source is given. */
    level?: number;
    /** A hex colour, or `rainbow` for the spectrum. */
    color?: string | "rainbow";
    cell?: number;
    /** Fires when the level crosses the live threshold. */
    onlive?: (live: boolean) => void;
    /** Report received levels and drawn frames to `MeterProbe` under this name. */
    probe?: string;
    class?: string;
  }

  let {
    source,
    level = 0,
    color = "rainbow",
    cell = 3,
    onlive,
    probe,
    class: className = "",
  }: Props = $props();

  let canvas: HTMLCanvasElement;
  let binding: LevelMeterBinding | null = null;

  $effect(() => {
    binding = new LevelMeterBinding(canvas, { source, color, cell, onLive: onlive, probe });
    return () => {
      binding?.destroy();
      binding = null;
    };
  });

  $effect(() => {
    if (binding && !source) binding.level = level;
  });
  $effect(() => {
    if (binding) binding.color = color;
  });

  onDestroy(() => binding?.destroy());
</script>

<canvas bind:this={canvas} class={className} data-rad-level></canvas>
