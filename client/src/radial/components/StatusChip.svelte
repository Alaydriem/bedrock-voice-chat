<script lang="ts">
  import type { Snippet } from "svelte";

  interface Props {
    /** muted is stopped on purpose, as opposed to broken — so it is not red. */
    severity?: "idle" | "ok" | "warn" | "bad" | "muted";
    children?: Snippet;
  }

  let { severity = "idle", children }: Props = $props();
</script>

<!-- State in one word. The dot is currentColor, so a severity sets the text and the
     dot together and they cannot end up disagreeing. -->
<span class="rad-status-chip {severity !== 'idle' ? `rad-status-chip--${severity}` : ''}">
  {@render children?.()}
</span>
