<script lang="ts">
    import type { PreflightStep } from "../../js/app/server/preflight/PreflightStep";
    import type { PreflightStepState } from "../../js/app/server/preflight/PreflightStepState";

    interface Props {
        steps: readonly PreflightStep[];
        /** True while checks are still running, so the total reads as unfinished. */
        checking: boolean;
        onopen: () => void;
    }
    let { steps, checking, onopen }: Props = $props();

    const BLOCK: Partial<Record<PreflightStepState, string>> = {
        running: "is-run",
        skipped: "is-skip",
        ok: "is-ok",
        warn: "is-warn",
        bad: "is-bad",
    };

    let total = $derived(steps.reduce((sum, step) => sum + step.ms, 0));
</script>

<!--
  One block per check and a total: the cheapest honest summary. You can see that something
  failed and roughly how far in without opening anything.
-->
<button class="rad-preflight" onclick={onopen} title="Open the preflight readout">
    <span class="rad-preflight__blocks">
        {#each steps as step (step.name)}
            <i class={BLOCK[step.state] ?? ""}></i>
        {/each}
    </span>
    <span class="rad-preflight__total">{checking ? "checking" : `${total} ms`}</span>
</button>
