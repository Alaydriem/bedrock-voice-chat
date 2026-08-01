<script lang="ts">
  import { type IconName, Icons } from "$radial/core/icons/Icons";

  interface Props {
    name: IconName;
    /** Size preset. Omit to inherit the host's font-size, which is the default. */
    size?: "sm" | "md" | "lg";
    /** Spin it, for something that is working. */
    spin?: boolean;
    class?: string;
  }

  let { name, size, spin = false, class: className = "" }: Props = $props();

  // Icons are decorative: the control around them carries the accessible name. An icon
  // that announced itself would double up with its own button's label.
  const markup = $derived(Icons.svg(name));
</script>

<span
  class="{size ? `rad-icon--${size}` : ''} {spin ? 'rad-icon-spin' : ''} {className}"
  data-rad-icon={name}>{@html markup}</span
>
