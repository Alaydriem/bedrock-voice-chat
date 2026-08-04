<script lang="ts">
    import Icon from "$radial/components/Icon.svelte";
    import ProximityRing from "$radial/components/ProximityRing.svelte";
    import { RosterRowView } from "../../js/app/server/RosterRowView";
    import type { ServerRosterEntry } from "../../js/app/server/ServerRosterEntry";
    import RadScreen from "../shell/RadScreen.svelte";
    import ServerRow from "./ServerRow.svelte";

    interface Props {
        entries: readonly ServerRosterEntry[];
        isRefreshing: boolean;
        appVersion: string;
        onchoose: (server: string) => void;
        onforget: (entry: ServerRosterEntry) => void;
        onadd: () => void;
        onrefresh: () => void;
        onsettings: () => void;
    }
    let {
        entries,
        isRefreshing,
        appVersion,
        onchoose,
        onforget,
        onadd,
        onrefresh,
        onsettings,
    }: Props = $props();

    /**
     * The row the ring is reading, or null when nothing is being pointed at.
     *
     * With nothing considered it rests on the row that can be joined rather than on
     * whichever server happens to be stored first, so the pane opens on the answer.
     */
    let considering = $state<ServerRosterEntry | null>(null);
    let reading = $derived(considering ?? RosterRowView.resting(entries));
    let view = $derived(reading ? RosterRowView.of(reading) : null);
</script>

<RadScreen label="Servers">
    <div class="rad-split">
        <div class="rad-visual-pane">
            <div class="rad-visual">
                <ProximityRing mode={view?.ring ?? "empty"} class="rad-ring--fill" />
                <span class="rad-caption">
                    <span class="rad-label">{reading ? reading.host : "No server"}</span>
                    <span class="rad-caption__value">{view?.caption ?? "NOTHING SAVED"}</span>
                </span>
            </div>
        </div>

        <div class="rad-content-pane rad-content-pane--top">
            <span class="rad-label rad-rise" style="--d: 50">Your servers</span>
            <h2 class="rad-display rad-rise" style="--d: 120; margin-top: 12px; font-size: 2rem">
                Choose where you're <b>playing.</b>
            </h2>
            <!--
              The list is checked as it draws, so the states below are worth explaining
              once rather than per row: a lapsed sign-in and a server that is down look
              similar in a list and are not the same problem.
            -->
            <p class="rad-body rad-rise" style="--d: 200">
                Each one is checked as it loads. A sign-in that has lapsed can be renewed
                here; a server that isn't answering is a question for whoever runs it.
            </p>

            <div class="rad-rise srv-list" style="--d: 280">
                {#each entries as entry (entry.server)}
                    <ServerRow
                        {entry}
                        onconsider={(e) => (considering = e)}
                        {onchoose}
                        {onforget}
                    />
                {/each}
            </div>

            <div class="rad-sheet__divider"></div>

            <button class="rad-list-row" onclick={onadd}>
                <span class="rad-list-row__icon"><Icon name="plus" /></span> Add a server
            </button>
            <button class="rad-list-row" onclick={onrefresh} disabled={isRefreshing}>
                <span class="rad-list-row__icon"><Icon name="refresh" spin={isRefreshing} /></span>
                {isRefreshing ? "Checking every server…" : "Check them again"}
            </button>
            <button class="rad-list-row" onclick={onsettings}>
                <span class="rad-list-row__icon"><Icon name="gear" /></span> Settings
            </button>
        </div>
    </div>

    {#snippet footbar()}
        <span class="rad-label">
            {entries.length}
            {entries.length === 1 ? "server saved" : "servers saved"}
        </span>
        <span class="rad-label rad-num">v{appVersion}</span>
    {/snippet}
</RadScreen>

<style>
    .srv-list {
        margin-top: 22px;
        display: flex;
        flex-direction: column;
        gap: 2px;
    }
</style>
