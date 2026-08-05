<script lang="ts">
    import { onDestroy, onMount } from "svelte";
    import Icon from "$radial/components/Icon.svelte";
    import Scope from "$radial/components/Scope.svelte";
    import Verdict from "$radial/components/Verdict.svelte";
    import { Diagnostics } from "$radial/core/controllers/Diagnostics";
    import { KvGridView } from "$radial/core/controllers/KvGridView";
    import type { LinkDiagnosticsSnapshot } from "../../js/bindings/LinkDiagnosticsSnapshot";
    import type { LinkHealth } from "../../js/app/dashboard/DiagnosticsManager";
    import { DiagnosticsView } from "../../js/app/dashboard/DiagnosticsView";

    interface Props {
        snapshot: LinkDiagnosticsSnapshot | null;
        health: LinkHealth;
        pttIdle: boolean;
        visiblePlayers: number;
        reconnecting: boolean;
        onreconnect: () => void;
        oncopy: () => void;
        /** Restarts every counter from now, for measuring a change against what it changed. */
        onreset: () => void;
        onclose: () => void;
    }
    let {
        snapshot,
        health,
        pttIdle,
        visiblePlayers,
        reconnecting,
        onreconnect,
        oncopy,
        onreset,
        onclose,
    }: Props = $props();

    const input = $derived(
        snapshot
            ? DiagnosticsView.input(snapshot, {
                  reconnecting: health.reconnecting,
                  attempt: health.attempt,
                  pttIdle,
                  visiblePlayers,
              })
            : null,
    );

    const verdict = $derived(input ? Diagnostics.verdict(input) : null);

    /** Seeded once from history, then advanced one sample per snapshot. */
    let seeded = $state<readonly number[]>([]);
    let ready = $state(false);

    $effect(() => {
        if (ready || !snapshot) return;
        seeded = DiagnosticsView.history(snapshot);
        ready = true;
    });

    /**
     * Reset, and re-seed the scope from whatever the backend reports next.
     *
     * The trace is seeded once from history and advanced a sample at a time after that, so a
     * reset that only zeroed the backend would leave seventy seconds of pre-reset round trips
     * drawn on screen — the one part of the panel still showing the old session.
     */
    function pressReset(): void {
        ready = false;
        seeded = [];
        onreset();
    }

    let gridHost: HTMLElement;
    let grid: KvGridView | null = null;

    onMount(() => {
        grid = new KvGridView(gridHost);
    });

    // Values written in place rather than markup rebuilt: this repaints once a second, and
    // replacing the DOM that often reflows the panel and reads as flicker.
    $effect(() => {
        if (!grid || !input || !snapshot) return;
        grid.update([...Diagnostics.groups(input), ...DiagnosticsView.extraGroups(snapshot)]);
    });

    onDestroy(() => {
        grid = null;
    });
</script>

<div class="rad-status" data-status>
    <div class="rad-status__head">
        <span class="rad-label">Status</span>
        <span class="rad-status__actions">
            <button class="rad-status__act" onclick={oncopy}>
                <Icon name="copy" /> Copy report
            </button>
            <button
                class="rad-status__act"
                title="Restart every counter from now"
                onclick={pressReset}
            >
                <Icon name="reset" /> Reset stats
            </button>
            <button
                class="rad-status__act"
                class:is-busy={reconnecting}
                disabled={reconnecting}
                onclick={onreconnect}
            >
                <Icon name="refresh" spin={reconnecting} />
                {reconnecting ? "Reconnecting…" : "Reconnect"}
            </button>
            <button class="rad-icon-btn" aria-label="Close status" onclick={onclose}>
                <Icon name="close" />
            </button>
        </span>
    </div>

    <div class="rad-status__body">
        <div class="rad-status__scope">
            {#if ready}
                <Scope bare history={seeded} sample={input?.rtt} />
            {:else}
                <Scope bare />
            {/if}
            <span class="rad-status__scope-cap">Round trip &middot; last 72 s</span>
        </div>

        <div class="rad-status__read">
            {#if verdict}
                <Verdict severity={verdict[0]} text={verdict[1]} />
            {:else}
                <!-- Absent rather than zeroed. A snapshot of zeros draws as a flawless link
                     with a 0 ms round trip, which misleads worse than saying nothing. -->
                <Verdict severity="warn" text="Not connected, so there is nothing to measure." />
            {/if}
            <div class="rad-kv-grid" bind:this={gridHost}></div>
        </div>
    </div>
</div>
