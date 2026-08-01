<script lang="ts">
  import Ring from "./Ring.svelte";
  import type { RingSource } from "$radial/core/ring/RingSource";

  interface Props {
    kicker?: string;
    headline?: string;
    note?: string;
    /** Empty is the at-rest ring. Pass sources to show the field is not silent. */
    sources?: readonly RingSource[];
    /** Fades out when someone arrives and the roster takes over. */
    gone?: boolean;
    size?: number;
  }

  let {
    kicker = "Listening",
    headline = "Nobody within 80 m",
    note = "Voices appear here the moment someone walks into range. Nothing to join, nothing to dial.",
    sources = [],
    gone = false,
    size = 330,
  }: Props = $props();
</script>

<!-- In a proximity app you are alone constantly: at connect, while mining, anywhere off
     the beaten path. This is not an edge case, it is somewhere users live, and a live
     ring says the system is on and listening without asking anyone to interpret it. -->
<div class="rad-idle" class:is-gone={gone}>
  <Ring mode={sources.length ? "live" : "empty"} {sources} {size} />
  <span class="rad-idle__caption">
    <span class="rad-idle__kicker">{kicker}</span>
    <span class="rad-idle__headline">{headline}</span>
    <span class="rad-idle__note">{note}</span>
  </span>
</div>
