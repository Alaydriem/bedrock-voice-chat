<script lang="ts">
  import { onDestroy } from "svelte";
  import { ScopeBinding } from "$radial/bindings/ScopeBinding";

  interface Props {
    /** Newest sample. Push a value here once per measurement tick. */
    sample?: number;
    samples?: number;
    unit?: string;
    warnAt?: number;
    faultAt?: number;
    size?: number;
    class?: string;
  }

  let {
    sample,
    samples = 72,
    unit = "MS RTT",
    warnAt = 60,
    faultAt = 90,
    size,
    class: className = "",
  }: Props = $props();

  let canvas: HTMLCanvasElement;
  let binding = $state<ScopeBinding | null>(null);

  $effect(() => {
    const b = new ScopeBinding(canvas, { samples, unit, warnAt, faultAt });
    binding = b;
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

<div
  class="rad-ring {size ? 'rad-ring--fixed' : ''} {className}"
  style={size ? `width:${size}px;height:${size}px` : undefined}
>
  <canvas bind:this={canvas} data-rad-ring="scope"></canvas>
</div>
