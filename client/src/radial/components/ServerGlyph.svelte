<script lang="ts">
  import { onDestroy } from "svelte";
  import { GlyphBinding } from "$radial/bindings/GlyphBinding";

  interface Props {
    /** Hostname or realm name. The glyph is derived from it and nothing else. */
    name: string;
    size?: number;
    class?: string;
  }

  let { name, size = 36, class: className = "" }: Props = $props();

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

  onDestroy(() => binding?.destroy());
</script>

<canvas bind:this={canvas} class={className} data-rad-glyph={name}></canvas>
