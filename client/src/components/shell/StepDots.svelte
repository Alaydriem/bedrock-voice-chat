<script lang="ts">
    interface Props {
        step: number;
        total: number;
        onselect?: (step: number) => void;
    }
    let { step, total, onselect }: Props = $props();

    const pad = (n: number) => String(n).padStart(2, "0");
    let dots = $derived(Array.from({ length: total }, (_, i) => i + 1));
</script>

<span class="rad-step-dots">
    {#each dots as n (n)}
        <button
            class={n === step ? "is-on" : ""}
            aria-label={`Step ${n}`}
            aria-current={n === step ? "step" : undefined}
            onclick={() => onselect?.(n)}
        ></button>
    {/each}
    <span class="rad-step-dots__count">{pad(step)} / {pad(total)}</span>
</span>
