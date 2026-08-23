<script lang="ts">
  import { onDestroy } from "svelte";
  import { RingBinding, type RingMode } from "$radial/bindings/RingBinding";
  import type { RingSource } from "$radial/core/ring/RingSource";
  import type { RingGeometry } from "$radial/core/ring/RingGeometry";

  interface Props {
    /** live · someone is talking. empty · nobody in range. lock · one source acquiring. */
    mode?: RingMode;
    /** Voices placed around the circle. */
    sources?: readonly RingSource[];
    /** Ring diameter against the canvas. */
    scale?: number;
    /** Mark size against the ring. Above 1 it overflows the hairline. */
    logoScale?: number;
    /** Rotation rate in radians per second. */
    spin?: number;
    /**
     * Mark amplitude, 0 to 1. Omit and the mode decides. Pass it when something is
     * measuring a level, so the mark filling out to its full silhouette is the reading.
     */
    gain?: number;
    /** Hold the bars at rest while the mark keeps moving. */
    ringStill?: boolean;
    /** An angular window removed from the ring: `[centre, half-width]` in radians. */
    cut?: readonly [centre: number, half: number];
    /** Colour flared at the two cut ends. */
    cutTone?: string;
    /** Paint each bar from the mark's own columns instead of one base colour. */
    spectrum?: boolean;
    /** Draw the mark at the centre. Off when something else occupies it. */
    mark?: boolean;
    /** Fixed size in px. Omit to fill the parent, which must be positioned. */
    size?: number;
    class?: string;
  }

  let {
    mode = "live",
    sources = [],
    scale = 1,
    logoScale = 1,
    spin = 0,
    gain,
    ringStill = false,
    cut,
    cutTone,
    spectrum = false,
    mark = true,
    size,
    class: className = "",
  }: Props = $props();

  let canvas: HTMLCanvasElement;
  let binding = $state<RingBinding | null>(null);

  // A cut, a tone, the spectrum and the mark are all fixed for the life of a screen, so
  // they remount the binding rather than being pushed onto it per frame like `gain`.
  $effect(() => {
    const b = new RingBinding(canvas, {
      mode,
      scale,
      logoScale,
      spin,
      gain,
      ringStill,
      cut,
      cutTone,
      spectrum,
      mark,
    });
    binding = b;
    return () => {
      b.destroy();
      binding = null;
    };
  });

  $effect(() => {
    if (binding) binding.mode = mode;
  });
  $effect(() => {
    if (binding) binding.gain = gain;
  });
  $effect(() => {
    binding?.setSources(sources);
  });

  /**
   * Geometry of the last painted frame. The handoff reads it so a card flies out from
   * where that player's bar actually was, rather than from the centre.
   */
  export function geometry(): RingGeometry | null {
    return binding?.geometry ?? null;
  }

  /**
   * The canvas the geometry is measured against.
   *
   * Both are needed together: the geometry is in canvas coordinates and the handoff flies in
   * viewport coordinates, so a bar's position is only recoverable from the pair.
   */
  export function element(): HTMLCanvasElement | null {
    return canvas ?? null;
  }

  onDestroy(() => binding?.destroy());
</script>

<div
  class="rad-ring {size ? 'rad-ring--fixed' : ''} {className}"
  style={size ? `width:${size}px;height:${size}px` : undefined}
>
  <canvas bind:this={canvas} data-rad-ring={mode}></canvas>
</div>
