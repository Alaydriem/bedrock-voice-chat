<script lang="ts">
  import type { Snippet } from "svelte";

  interface Props {
    severity?: "default" | "warn" | "bad";
    /** The onboarding variant: brand-tinted, a reassurance rather than a caveat. */
    aside?: boolean;
    children?: Snippet;
  }

  let { severity = "default", aside = false, children }: Props = $props();
</script>

<!-- Explains. No action and no dismiss: a fact about how the product works does not
     stop being true. For something that needs a button, use Banner. -->
<div
  class="rad-callout {severity !== 'default' ? `rad-callout--${severity}` : ''} {aside
    ? 'rad-callout--aside'
    : ''}"
>
  <span>{@render children?.()}</span>
</div>
