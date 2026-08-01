<script lang="ts">
  import type { Snippet } from "svelte";
  import Mark from "./Mark.svelte";

  interface Props {
    title: string;
    note?: string;
    /** The mark as a watermark. Off for a failure, which is not a resting state. */
    watermark?: boolean;
    actions?: Snippet;
  }

  let { title, note, watermark = true, actions }: Props = $props();
</script>

<!-- Loading, failed and empty are three different screens. This is only the third:
     nothing is here yet, and here is how something gets here. A failure needs a retry
     and a reason; a load needs a skeleton. -->
<div class="rad-empty">
  {#if watermark}<Mark cell={4} gain={0.35} still />{/if}
  <span class="rad-empty__title">{title}</span>
  {#if note}<span class="rad-empty__note">{note}</span>{/if}
  {#if actions}<span class="rad-swatchrow" style="justify-content:center">{@render actions()}</span>{/if}
</div>
