<script lang="ts">
    import Icon from "$radial/components/Icon.svelte";
    import Mark from "$radial/components/Mark.svelte";
    import { PlateView } from "../../js/app/server/PlateView";
    import type { ServerRosterEntry } from "../../js/app/server/ServerRosterEntry";
    import ServerPlate from "./ServerPlate.svelte";

    interface Props {
        entries: readonly ServerRosterEntry[];
        isRefreshing: boolean;
        onchoose: (server: string) => void;
        onopen: (server: string) => void;
        onadd: () => void;
        onrecheckall: () => void;
    }
    let { entries, isRefreshing, onchoose, onopen, onadd, onrecheckall }: Props = $props();

    const SEVERITY_DOT: Record<string, string> = {
        ok: "var(--color-rad-ok)",
        warn: "var(--color-rad-warn)",
        bad: "var(--color-rad-fault)",
        busy: "var(--color-rad-brand-lift)",
    };

    let tally = $derived(PlateView.tally(entries));
</script>

<!--
  Not RadScreen: this screen's top bar carries controls rather than a label, and its body is
  a head plus a scrolling list rather than a split. The ring is deliberately absent — it is
  the empty state and the status oscilloscope, never a roster, because picking a server wants
  recognition rather than recall.
-->
<section class="rad-screen is-on">
    <div class="rad-topbar">
        <span class="rad-brand">
            <Mark />
            <span class="rad-wordmark">Bedrock Voice Chat</span>
        </span>
        <span class="rad-footbar__actions">
            <button
                class="rad-icon-btn"
                onclick={onrecheckall}
                disabled={isRefreshing}
                title="Recheck every server"
                aria-label="Recheck every server"
            >
                <Icon name="refresh" spin={isRefreshing} />
            </button>
            <button class="rad-btn" onclick={onadd}>
                <Icon name="plus" /> Add a server
            </button>
        </span>
    </div>

    <div class="rad-server-head">
        <span class="rad-label">Select a server</span>
        <span class="rad-server-head__line"></span>
        <span class="rad-server-head__count">
            {entries.length}
            {entries.length === 1 ? "server" : "servers"}
        </span>
    </div>

    <div class="rad-server-list">
        <div class="rad-server-grid">
            {#each entries as entry, index (entry.server)}
                <ServerPlate {entry} {index} {onchoose} {onopen} />
            {/each}

            <!--
              A tile in the same grid rather than a button in the header alone: adding a
              server is one of the things you can pick on a screen whose whole job is picking.
            -->
            <button class="rad-server-add" onclick={onadd}>
                <Icon name="plus" />
                <span class="rad-server-add__label">Add a server</span>
            </button>
        </div>
    </div>

    <div class="rad-footbar">
        <span class="rad-server-tally">
            {#each tally as item (item.label)}
                <span class="rad-server-tally__item">
                    <i style="background: {SEVERITY_DOT[item.severity]}"></i>{item.count}
                    {item.label}
                </span>
            {/each}
        </span>
        <span class="rad-footbar__actions">
            <button class="rad-btn" onclick={onrecheckall} disabled={isRefreshing}>
                <Icon name="refresh" spin={isRefreshing} />
                {isRefreshing ? "Rechecking…" : "Recheck all"}
            </button>
        </span>
    </div>
</section>
