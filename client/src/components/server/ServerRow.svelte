<script lang="ts">
    import Icon from "$radial/components/Icon.svelte";
    import ServerGlyph from "$radial/components/ServerGlyph.svelte";
    import StatusChip from "$radial/components/StatusChip.svelte";
    import { RosterRowView } from "../../js/app/server/RosterRowView";
    import type { ServerRosterEntry } from "../../js/app/server/ServerRosterEntry";

    interface Props {
        entry: ServerRosterEntry;
        /** Raised on hover and on focus, so the ring answers a keyboard too. */
        onconsider?: (entry: ServerRosterEntry | null) => void;
        onchoose: (server: string) => void;
        onforget: (entry: ServerRosterEntry) => void;
    }
    let { entry, onconsider, onchoose, onforget }: Props = $props();

    let view = $derived(RosterRowView.of(entry));
    let explanation = $derived(entry.note || view.blocked);

    /**
     * Tabbing from Join to Forget within one row is not leaving the row, and treating it as
     * such would drop the ring back to its resting state and then pick it up again.
     */
    function released(event: FocusEvent): void {
        const row = event.currentTarget as HTMLElement;
        const next = event.relatedTarget as Node | null;
        if (!next || !row.contains(next)) onconsider?.(null);
    }
</script>

<div
    class="rad-sheet-row srv-row"
    class:is-on={entry.isCurrent}
    onmouseenter={() => onconsider?.(entry)}
    onmouseleave={() => onconsider?.(null)}
    onfocusin={() => onconsider?.(entry)}
    onfocusout={released}
>
    <ServerGlyph name={entry.host} size={30} />

    <span class="rad-sheet-row__text srv-row__text">
        <span class="rad-sheet-row__name">{entry.host}</span>
        <span class="rad-sheet-row__host">{entry.player} &middot; {entry.game}</span>
        {#if explanation}
            <span class="rad-row__note">{explanation}</span>
        {/if}
    </span>

    <span class="srv-row__end">
        <StatusChip severity={view.severity}>{view.status}</StatusChip>

        {#if view.action}
            <button
                class="rad-btn {RosterRowView.isJoinable(entry) ? 'rad-btn--primary' : ''}"
                onclick={() => onchoose(entry.server)}
            >
                {view.action}
            </button>
        {/if}

        <!--
          Forgetting is deliberately not the same shape as joining. It is an icon in the
          quiet row-action style, and it opens a confirm rather than acting.
        -->
        <button
            class="rad-kebab"
            onclick={() => onforget(entry)}
            aria-label="Forget {entry.host}"
            title="Forget this server"
        >
            <Icon name="trash" />
        </button>
    </span>
</div>

<style>
    /**
     * The kit's sheet row is a glyph, a name and a tick. This one carries a state and two
     * actions as well, so the text column takes the slack and the controls hold their size.
     */
    .srv-row {
        align-items: center;
        cursor: default;
    }

    .srv-row:hover {
        background: transparent;
    }

    .srv-row.is-on:hover {
        background: var(--color-rad-panel);
    }

    .srv-row__text {
        flex: 1 1 auto;
    }

    .srv-row__end {
        flex: 0 0 auto;
        display: flex;
        align-items: center;
        gap: 8px;
        margin-left: auto;
    }

    /* A row whose state and actions no longer fit beside its name stacks instead of
       truncating: the address is what identifies a server and must stay readable. */
    @container rad (max-width: 560px) {
        .srv-row {
            flex-wrap: wrap;
        }

        .srv-row__end {
            width: 100%;
            margin-left: 42px;
        }
    }
</style>
