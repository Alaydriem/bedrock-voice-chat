<script lang="ts">
  import { onDestroy } from "svelte";
  import { GlyphBinding } from "$radial/bindings/GlyphBinding";

  interface Props {
    /** Hostname or realm name. The glyph is derived from it and nothing else. */
    name: string;
    size?: number;
    /**
     * Draw the glyph in over its own reveal rather than appearing at once, so a list of
     * servers assembles. Milliseconds; 0 draws immediately.
     */
    reveal?: number;
    class?: string;
  }

  let { name, size = 36, reveal = 0, class: className = "" }: Props = $props();

  let canvas: HTMLCanvasElement;
  let binding: GlyphBinding | null = null;

  $effect(() => {
    binding = new GlyphBinding(canvas, name, { size });
    return () => {
      binding?.destroy();
      binding = null;
    };
  });

  // Drawn once per name, never on the animation loop: a rail of servers repainting
  // every frame would cost more than the rest of the dashboard and look identical.
  $effect(() => {
    if (binding) binding.name = name;
  });

  /**
   * The reveal is a one-shot rather than a loop registration, so it ends on its own and
   * leaves the glyph static — which is what everything else on this component assumes.
   */
  $effect(() => {
    if (!binding || reveal <= 0) return;
    const target = binding;
    const started = performance.now();
    let frame = requestAnimationFrame(function tick(now) {
      const progress = Math.min(1, (now - started) / reveal);
      target.render(progress);
      if (progress < 1) frame = requestAnimationFrame(tick);
    });
    return () => cancelAnimationFrame(frame);
  });

  onDestroy(() => binding?.destroy());
</script>

<canvas bind:this={canvas} class={className} data-rad-glyph={name}></canvas>
