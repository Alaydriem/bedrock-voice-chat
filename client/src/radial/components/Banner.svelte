<script lang="ts">
  import type { Snippet } from "svelte";
  import Icon from "./Icon.svelte";
  import type { IconName } from "$radial/core/icons/Icons";

  interface Props {
    severity?: "default" | "warn" | "bad";
    icon?: IconName;
    /** The action. A banner without one should have been a callout. */
    action?: Snippet;
    children?: Snippet;
  }

  let { severity = "default", icon, action, children }: Props = $props();

  const fallback: Record<string, IconName> = { default: "info", warn: "warn", bad: "close" };
</script>

<!-- Page-level state that needs a decision, pinned above the content rather than
     inline with a setting. Not dismissible: dismissing it would not restart the app,
     grant the microphone permission, or renew the session. -->
<div class="rad-banner {severity !== 'default' ? `rad-banner--${severity}` : ''}">
  <span class="rad-banner__icon"><Icon name={icon ?? fallback[severity]} /></span>
  <span class="rad-banner__text">{@render children?.()}</span>
  {@render action?.()}
</div>
