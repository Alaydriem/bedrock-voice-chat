<script lang="ts">
  import { onDestroy, type Snippet } from "svelte";
  import { AnimationLoop } from "$radial/core/canvas/AnimationLoop";
  import { IntroSequence } from "$radial/core/intro/IntroSequence";
  import { LoaderStatus } from "$radial/core/intro/LoaderStatus";

  interface Props {
    /** While true the mark dances. Flip to false and it collapses to its flat row. */
    loading?: boolean;
    /** Play the full boot sequence before settling into the dance. */
    withIntro?: boolean;
    /**
     * A fixed size in CSS px. Omit and the stylesheet owns the size, which is what
     * every screen wants — a 330px mark is most of a phone and a tenth of a desktop
     * window, so a fixed number cannot be right in both places.
     */
    size?: number;
    /** Null keeps the alpha channel, so the loader sits on whatever is behind it. */
    background?: string | null;
    /** Fires once the collapse has landed. */
    onfinished?: () => void;
    /** Cycled beneath the mark once the wait is long enough. Omit for silence. */
    phrases?: readonly string[];
    slowAfterSeconds?: number;
    /** Rendered below the status line, for a recovery affordance on a long wait. */
    children?: Snippet;
  }

  let {
    loading = true,
    withIntro = false,
    size,
    background = null,
    onfinished,
    phrases = [],
    slowAfterSeconds = 4,
    children,
  }: Props = $props();

  let canvas: HTMLCanvasElement;
  let sequence: IntroSequence | null = null;
  let collapsing = false;

  let visible = $state(false);
  let glyph = $state("");
  let phrase = $state("");

  $effect(() => {
    sequence = new IntroSequence(
      canvas,
      size === undefined
        ? { fluid: true, background }
        : { width: size, height: size, background },
    );
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

  // Stepped from the kit's shared rAF loop rather than an interval: motion has to
  // survive Android WebView settings that suppress CSS animation, and the mark
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

  onDestroy(() => sequence?.stop());
</script>

<div class="rad-loader">
  <canvas bind:this={canvas}></canvas>
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
