<script lang="ts">
  import type { Snippet } from "svelte";
  import { type IconName, Icons } from "$radial/core/icons/Icons";

  interface Props {
    /** md is the settings scale; lg is the tracked, uppercase onboarding scale. */
    size?: "md" | "lg";
    variant?: "default" | "primary" | "quiet" | "danger";
    icon?: IconName;
    disabled?: boolean;
    type?: "button" | "submit";
    onclick?: (e: MouseEvent) => void;
    class?: string;
    children?: Snippet;
  }

  let {
    size = "md",
    variant = "default",
    icon,
    disabled = false,
    type = "button",
    onclick,
    class: className = "",
    children,
  }: Props = $props();

  const classes = $derived(
    ["rad-btn", size === "lg" ? "rad-btn--lg" : "", variant !== "default" ? `rad-btn--${variant}` : "", className]
      .filter(Boolean)
      .join(" "),
  );
</script>

<button class={classes} {type} {disabled} {onclick}>
  {#if icon}<span data-rad-icon={icon}>{@html Icons.svg(icon)}</span>{/if}
  {@render children?.()}
</button>
