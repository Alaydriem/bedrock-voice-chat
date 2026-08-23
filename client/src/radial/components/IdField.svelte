<script lang="ts">
  import { onDestroy, type Snippet } from "svelte";
  import Icon from "./Icon.svelte";

  interface Props {
    /** Shown whole, wrapping if it has to. */
    value: string;
    /** Above the field. Leave out where the row around it already names the value. */
    label?: string;
    /** For the copy button, which has no visible text. */
    copyLabel?: string;
    /** Right of the copy button — the About pane puts Refresh here. */
    actions?: Snippet;
  }
  let { value, label, copyLabel = "Copy", actions }: Props = $props();

  let copied = $state(false);
  let copiedTimer: ReturnType<typeof setTimeout> | null = null;

  // The tick replaces the copy mark rather than joining it. A confirmation beside the
  // control it confirms widens the field, and the value beside it reflows.
  async function copy(): Promise<void> {
    try {
      await navigator.clipboard?.writeText(value);
      copied = true;
      if (copiedTimer) clearTimeout(copiedTimer);
      copiedTimer = setTimeout(() => (copied = false), 1500);
    } catch (_) {}
  }

  onDestroy(() => {
    if (copiedTimer) clearTimeout(copiedTimer);
  });
</script>

<!-- One value someone else needs to read: an id, a key, a reference. Wraps rather than
     clips, because two thirds of a uuid identifies nothing. -->
{#if label}
  <span class="rad-label" style="display: block">{label}</span>
{/if}
<div class="rad-idfield" style={label ? "margin-top: 8px" : undefined}>
  <span class="rad-idfield__value">{value}</span>
  <span class="rad-idfield__actions">
    <button class="rad-icon-btn" onclick={() => void copy()} aria-label={copyLabel}>
      <Icon name={copied ? "check" : "copy"} />
    </button>
    {#if actions}{@render actions()}{/if}
  </span>
</div>
