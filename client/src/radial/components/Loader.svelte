<script lang="ts">
  import { type Snippet } from "svelte";
  import ProximityRing from "./ProximityRing.svelte";
  import { AnimationLoop } from "$radial/core/canvas/AnimationLoop";
  import { LoaderStatus } from "$radial/core/intro/LoaderStatus";

  interface Props {
    /** While true the ring is live and the mark dances. Off holds it empty. */
    loading?: boolean;
    /**
     * A fixed size in CSS px. Omit and the stylesheet owns the size, which is what
     * every screen wants — a 330px mark is most of a phone and a tenth of a desktop
     * window, so a fixed number cannot be right in both places.
     */
    size?: number;
    /**
     * Mark amplitude while it dances, 0 to 1.
     *
     * Full by default: this is the same object the introduction, the gate and a resolved
     * address show, and it should not be recognisably quieter here. Lower it for a screen
     * that wants the mark to defer to something beside it.
     */
    idleGain?: number;
    /** Cycled beneath the mark once the wait is long enough. Omit for silence. */
    phrases?: readonly string[];
    slowAfterSeconds?: number;
    /** Rendered below the status line, for a recovery affordance on a long wait. */
    children?: Snippet;
  }

  let {
    loading = true,
    size,
    idleGain = 1,
    phrases = [],
    slowAfterSeconds = 4,
    children,
  }: Props = $props();

  let visible = $state(false);
  let glyph = $state("");
  let phrase = $state("");

  // Stepped from the kit's shared rAF loop rather than an interval: motion has to
  // survive Android WebView settings that suppress CSS animation, and the ring
  // already depends on this loop, so a second timer buys nothing.
  $effect(() => {
    if (!loading || phrases.length === 0) {
      visible = false;
      return;
    }

    const status = new LoaderStatus({ phrases, slowAfterSeconds });
    let start: number | null = null;

    return AnimationLoop.shared().add((t) => {
      start ??= t;
      const frame = status.at((t - start) / 1000);
      visible = frame.visible;
      glyph = frame.glyph;
      phrase = frame.phrase;
    });
  });
</script>

<!--
  A wait, drawn as the product's own object: `ProximityRing`, the same component the
  introduction's proximity step, the gate and a resolved address show.
-->
<div class="rad-loader">
  <ProximityRing mode={loading ? "live" : "empty"} gain={idleGain} {size} />
  {#if visible}
    <p class="rad-loader__status" role="status" aria-live="polite">
      <span class="rad-loader__glyph" aria-hidden="true">{glyph}</span>
      <span>{phrase}</span>
    </p>
    {#if children}
      <div class="rad-loader__aside">{@render children()}</div>
    {/if}
  {/if}
</div>
