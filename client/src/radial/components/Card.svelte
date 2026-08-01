<script lang="ts">
  import type { Snippet } from "svelte";

  interface Props {
    /** Mono caption above the rows. Omit for a card that needs no heading. */
    title?: string;
    /** Pad the body. Off when the card holds rows or a table, which pad themselves. */
    padded?: boolean;
    class?: string;
    children?: Snippet;
  }

  let { title, padded = false, class: className = "", children }: Props = $props();
</script>

<div class="rad-card {className}">
  {#if title}<div class="rad-card__head">{title}</div>{/if}
  {#if padded}
    <div class="rad-card__body">{@render children?.()}</div>
  {:else}
    {@render children?.()}
  {/if}
</div>
