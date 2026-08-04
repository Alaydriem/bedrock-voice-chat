<script lang="ts">
    import StatusChip from "$radial/components/StatusChip.svelte";
    import { ServerGlyph } from "$radial/core/glyph/ServerGlyph";
    import { PlateView } from "../../js/app/server/PlateView";
    import type { ServerRosterEntry } from "../../js/app/server/ServerRosterEntry";
    import PreflightStrip from "./PreflightStrip.svelte";
    import ServerIdentity from "./ServerIdentity.svelte";

    interface Props {
        entry: ServerRosterEntry;
        /** Entrance stagger, so the list assembles rather than appears. */
        index: number;
        onchoose: (server: string) => void;
        onopen: (server: string) => void;
    }
    let { entry, index, onchoose, onopen }: Props = $props();

    /** Long enough to read as the plate assembling, short enough not to delay recognition. */
    const GLYPH_REVEAL_MS = 420;

    let view = $derived(PlateView.of(entry));
    let hue = $derived(ServerGlyph.of(entry.host).hue);
    let blocked = $derived(view.kind === "blocked");

    /**
     * The plate the pointer is over and the plate the keyboard is on are the same state: one
     * server under consideration. Tabbing between this plate's two controls is not leaving
     * it, so the release checks where focus actually went.
     */
    let considering = $state(false);

    function released(event: FocusEvent): void {
        const plate = event.currentTarget as HTMLElement;
        const next = event.relatedTarget as Node | null;
        if (!next || !plate.contains(next)) considering = false;
    }
</script>

<div
    class="rad-server rad-rise"
    style="--d: {index * 60}"
    class:is-focus={considering}
    onmouseenter={() => (considering = true)}
    onmouseleave={() => (considering = false)}
    onfocusin={() => (considering = true)}
    onfocusout={released}
>
    <!--
      Operator art when it exists, the server's own derived hue when it does not. Never a
      grey box: an absent asset should still look like this product.
    -->
    {#if entry.canvasUrl}
        <span class="rad-server__art" style="background-image: url({entry.canvasUrl})">
            <ServerIdentity
                host={entry.host}
                avatarUrl={entry.avatarUrl}
                size={52}
                reveal={GLYPH_REVEAL_MS}
                class="rad-server__id"
            />
        </span>
    {:else}
        <span class="rad-server__art rad-server__art--derived" style="--rad-server-hue: {hue}">
            <ServerIdentity
                host={entry.host}
                avatarUrl={entry.avatarUrl}
                size={52}
                reveal={GLYPH_REVEAL_MS}
                class="rad-server__id"
            />
        </span>
    {/if}

    <span class="rad-server__body">
        <!--
          The host is the name line because nothing supplies a friendlier one: the plate
          reserves that slot for an operator-declared name the product does not have yet.
        -->
        <span class="rad-server__name">{entry.host}</span>
        <span class="rad-server__host">signed in as {entry.player}</span>
        <span class="rad-server__state">
            <StatusChip severity={view.severity}>{view.chip}</StatusChip>
        </span>
        {#if entry.note}
            <span class="rad-row__note">{entry.note}</span>
        {/if}
    </span>

    <span class="rad-server__foot">
        <PreflightStrip
            steps={entry.steps}
            checking={entry.status === "checking"}
            onopen={() => onopen(entry.server)}
        />
        <button
            class="rad-btn {blocked ? '' : 'rad-btn--primary'}"
            onclick={() => onchoose(entry.server)}
            disabled={blocked}
        >
            {view.action}
        </button>
    </span>
</div>
