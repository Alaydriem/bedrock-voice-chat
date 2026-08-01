<script lang="ts">
  interface Props {
    checked?: boolean;
    disabled?: boolean;
    /** Names the control for a screen reader when there is no visible label. */
    label?: string;
    onchange?: (checked: boolean) => void;
  }

  let { checked = $bindable(false), disabled = false, label, onchange }: Props = $props();

  function toggle() {
    if (disabled) return;
    checked = !checked;
    onchange?.(checked);
  }
</script>

<!-- role=switch rather than checkbox: this is on/off, not selected/unselected, and
     screen readers announce it that way. -->
<button
  class="rad-toggle"
  type="button"
  role="switch"
  aria-checked={checked}
  aria-label={label}
  {disabled}
  onclick={toggle}
></button>
