<script lang="ts">
  interface Option {
    value: string;
    label: string;
  }

  interface Props {
    options: readonly Option[];
    value?: string;
    /** Names the group for a screen reader. */
    label?: string;
    onchange?: (value: string) => void;
  }

  let { options, value = $bindable(options[0]?.value ?? ""), label, onchange }: Props = $props();

  function pick(next: string) {
    value = next;
    onchange?.(next);
  }
</script>

<!-- Two or three short words. When each option needs a sentence it becomes a radio
     group, because a segment cannot hold an explanation. -->
<span class="rad-segmented" role="group" aria-label={label}>
  {#each options as option (option.value)}
    <button type="button" aria-pressed={value === option.value} onclick={() => pick(option.value)}>
      {option.label}
    </button>
  {/each}
</span>
