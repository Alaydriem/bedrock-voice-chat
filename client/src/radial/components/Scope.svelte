<script lang="ts">
  import { onDestroy } from "svelte";
  import { ScopeBinding } from "$radial/bindings/ScopeBinding";

  interface Props {
    /** Newest sample. Push a value here once per measurement tick. */
    sample?: number;
    /**
     * Samples already measured, oldest first, seeded once on mount.
     *
     * A caller holding history can open on a real trace rather than filling one in over the
     * next seventy seconds. It cannot be a reactive push: a prop delivers one value, and a
     * history is seventy sequential ones.
     */
    history?: readonly number[];
    samples?: number;
    unit?: string;
    warnAt?: number;
    faultAt?: number;
    size?: number;
    /**
     * Render the canvas alone, with no `.rad-ring` wrapper.
     *
     * For a host that sizes the canvas itself — `.rad-status__scope` does, and at two
     * breakpoints. The wrapper exists to give a fill-the-parent ring a positioning context,
     * and inside such a host it does the opposite: `.rad-ring > canvas` is
     * `position: absolute`, so an unsized wrapper lifts the scope out of the layout and paints
     * it over whatever happens to be underneath.
     */
    bare?: boolean;
    class?: string;
  }

  let {
    sample,
    history = [],
    samples = 72,
    unit = "MS RTT",
    warnAt = 60,
    faultAt = 90,
    size,
    bare = false,
    class: className = "",
  }: Props = $props();

  let canvas: HTMLCanvasElement;
  let binding = $state<ScopeBinding | null>(null);

  $effect(() => {
    const b = new ScopeBinding(canvas, { samples, unit, warnAt, faultAt });
    binding = b;
    for (const value of history) b.push(value);
    return () => {
      b.destroy();
      binding = null;
    };
  });

  // Each distinct value is one bar. The scope is a history, so a repeated identical
  // reading still has to advance the write head.
  $effect(() => {
    if (binding && sample !== undefined) binding.push(sample);
  });

  export function reset(): void {
    binding?.reset();
  }

  onDestroy(() => binding?.destroy());
</script>

{#if bare}
  <canvas bind:this={canvas} data-rad-ring="scope" class={className}></canvas>
{:else}
  <div
    class="rad-ring {size ? 'rad-ring--fixed' : ''} {className}"
    style={size ? `width:${size}px;height:${size}px` : undefined}
  >
    <canvas bind:this={canvas} data-rad-ring="scope"></canvas>
  </div>
{/if}
