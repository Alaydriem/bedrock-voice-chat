<script lang="ts">
  import Ring from "./Ring.svelte";
  import { AnimationLoop } from "$radial/core/canvas/AnimationLoop";
  import { ProximityCast } from "$radial/core/sources/ProximityCast";
  import type { RingMode } from "$radial/bindings/RingBinding";
  import type { RingSource } from "$radial/core/ring/RingSource";

  interface Props {
    /**
     * Which ring state to draw.
     *
     * The cast is placed on anything that is not `empty`. `lock` is a ring that has found
     * something — a resolved address, a handshake in flight — so it carries voices too;
     * only `empty` means nobody is there, and only it holds the ring still.
     */
    mode?: RingMode;
    /** How many of the cast to place. Four is what fits a pane beside a column of copy. */
    count?: number;
    /** Mark amplitude, 0 to 1. */
    gain?: number;
    /** A fixed size in CSS px. Omit to fill the parent, which must be positioned. */
    size?: number;
    /** How many are audible this frame, for a caption that counts them. */
    onaudible?: (audible: number) => void;
    class?: string;
  }

  let {
    mode = "live",
    count = 4,
    gain,
    size,
    onaudible,
    class: className = "",
  }: Props = $props();

  let sources = $state<RingSource[]>([]);

  /**
   * How alive the ring is, 0 to 1, eased toward its target each frame.
   *
   * A hard switch cuts the colour and the amplitude in one frame, which reads as a glitch
   * rather than as an answer. Fading the voices first and letting the mode carry the colour
   * afterwards makes it land as a decay. Down faster than up: "there is nobody there" should
   * arrive immediately, and coming back to life can afford to bloom.
   */
  let alive = $state(1);
  const FLOOR = 0.02;

  $effect(() =>
    AnimationLoop.shared().add((t) => {
      const target = mode === "empty" ? FLOOR : 1;
      alive += (target - alive) * (target < alive ? 0.22 : 0.12);

      if (alive <= FLOOR + 0.001) {
        if (sources.length > 0) sources = [];
        onaudible?.(0);
        return;
      }

      const placed = ProximityCast.placements(t, count);
      if (alive < 1) for (const source of placed) source.volume *= alive;
      sources = placed;
      onaudible?.(placed.length);
    }),
  );
</script>

<!--
  The product's signature object: people placed around you, talking, at distances that
  drift. One component for every surface that shows it — the introduction's proximity step,
  the gate, a resolved address, and a wait — so they are the same object rather than four
  that resemble each other.

  Positional rather than synthetic, which is also what makes it cheap: falloff is quadratic,
  so most of the cast is quiet most of the time, and a quiet voice draws fewer segments.
-->
<Ring {mode} {sources} {gain} {size} class={className} />
