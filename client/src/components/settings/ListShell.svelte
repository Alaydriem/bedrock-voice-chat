<script lang="ts">
    import type { Snippet } from "svelte";
    import Mark from "$radial/components/Mark.svelte";
    import type { ListState } from "../../js/app/settings/ListState";

    interface Props {
        state: ListState;
        /** How many rows the ready state actually has. Zero is its own screen. */
        count: number;
        failTitle: string;
        failNote: string;
        retryLabel?: string;
        emptyTitle: string;
        emptyNote: string;
        onretry?: () => void;
        /** Shown under the empty state, where there is something the reader can do. */
        emptyAction?: Snippet;
        children: Snippet;
    }
    let {
        state,
        count,
        failTitle,
        failNote,
        retryLabel = "Try again",
        emptyTitle,
        emptyNote,
        onretry,
        emptyAction,
        children,
    }: Props = $props();
</script>

<!-- Loading, failed and empty are three screens. Rows never render over a failure. -->
{#if state === "loading"}
    <div class="rad-card">
        <div class="rad-card__body" style="display: flex; flex-direction: column; gap: 13px">
            <span class="rad-skeleton" style="width: 62%"></span>
            <span class="rad-skeleton" style="width: 84%"></span>
            <span class="rad-skeleton" style="width: 41%"></span>
        </div>
    </div>
{:else if state === "failed"}
    <div class="rad-card">
        <div class="rad-empty" style="padding: 30px 20px">
            <span class="rad-empty__title">{failTitle}</span>
            <span class="rad-empty__note">{failNote}</span>
            {#if onretry}
                <span class="rad-swatchrow" style="justify-content: center">
                    <button class="rad-btn rad-btn--primary" onclick={onretry}>{retryLabel}</button>
                </span>
            {/if}
        </div>
    </div>
{:else if count === 0}
    <div class="rad-card">
        <div class="rad-empty" style="padding: 26px 20px">
            <Mark cell={4} gain={0.35} still />
            <span class="rad-empty__title">{emptyTitle}</span>
            <span class="rad-empty__note">{emptyNote}</span>
            {#if emptyAction}
                <span class="rad-swatchrow" style="justify-content: center">
                    {@render emptyAction()}
                </span>
            {/if}
        </div>
    </div>
{:else}
    {@render children()}
{/if}
