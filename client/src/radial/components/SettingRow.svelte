<script lang="ts">
  import type { Snippet } from "svelte";

  interface Props {
    label: string;
    /** One sentence. Anything longer belongs in a callout under the card. */
    note?: string;
    /** Put the control below the text even on a wide container — sliders, long paths. */
    stack?: boolean;
    control?: Snippet;
    children?: Snippet;
  }

  let { label, note, stack = false, control, children }: Props = $props();
</script>

<!-- The row is the unit of settings: a label, at most one sentence, one control. On a
     narrow container the control drops below the text rather than being crushed
     against it, and that is the only structural change the whole kit makes. -->
<div class="rad-row" class:rad-row--stack={stack}>
  <span class="rad-row__text">
    <span class="rad-row__label">{label}</span>
    {#if note}<span class="rad-row__note">{note}</span>{/if}
  </span>
  {#if control}
    <span class="rad-row__control">{@render control()}</span>
  {:else}
    {@render children?.()}
  {/if}
</div>
