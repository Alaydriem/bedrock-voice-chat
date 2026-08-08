<script lang="ts">
  import { I18n } from "$lib/i18n";
    import { invoke } from "@tauri-apps/api/core";
    import type { UnlistenFn } from "@tauri-apps/api/event";
    import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
    import { error } from "@tauri-apps/plugin-log";
    import { onDestroy, onMount } from "svelte";
    import { Coalescer } from "../../../js/app/utils/Coalescer";
    import Icon from "$radial/components/Icon.svelte";
    import Segmented from "$radial/components/Segmented.svelte";
    import ServerGlyph from "$radial/components/ServerGlyph.svelte";
    import StatusChip from "$radial/components/StatusChip.svelte";
    import type { PlayerSettingsRow } from "../../../js/bindings/PlayerSettingsRow";
    import type { PlayerRow } from "../../../js/app/settings/PlayerRow";
    import { PlayersView, type PlayerScope } from "../../../js/app/settings/PlayersView";
    import type { ListState } from "../../../js/app/settings/ListState";
    import ListShell from "../ListShell.svelte";

    let listState = $state<ListState>("loading");
    let all = $state<readonly PlayerRow[]>([]);
    let failure = $state("");

    let scope = $state<PlayerScope>("adjusted");
    let query = $state("");
    let page = $state(0);
    let resetting = $state(false);

    /**
     * The row the pointer is currently on, kept visible whatever the filter says.
     *
     * Without this, dragging a player back to exactly 100% under the Adjusted segment makes
     * the row stop being "adjusted" mid-drag, the keyed `{#each}` destroys the range input
     * under the pointer, and the drag dies. 1.00 is a valid step, and going back to normal is
     * the single most likely thing somebody does on this screen.
     */
    let holding = $state<string | null>(null);

    const matching = $derived(PlayersView.matching(all, scope, query, holding));
    const pages = $derived(PlayersView.pageCount(matching));
    const pageSlots = $derived(PlayersView.pageWindow(page, pages));
    const shown = $derived(PlayersView.page(matching, page));
    const empty = $derived(PlayersView.empty(scope, query));
    const meta = $derived(PlayersView.meta(all, scope));
    const adjustedCount = $derived(all.filter((row) => row.adjusted).length);

    async function load(): Promise<void> {
        try {
            all = PlayersView.rows(await invoke<PlayerSettingsRow[]>("player_settings_list"));
            listState = "ready";
        } catch (e) {
            // A rejected invoke can carry an empty string, which would render a titled failure
            // card with nothing under it. Every sibling pane supplies a fallback for the same
            // reason.
            const detail = e instanceof Error ? e.message : String(e);
            failure = detail.trim() || "The settings file could not be read on this device.";
            listState = "failed";
        }
    }

    /**
     * Re-read when the backend changes a setting underneath us.
     *
     * The in-game `/bvc volume` and `/bvc mute` write through the same coordinator and emit
     * this event. Without listening, the pane shows a stale row — and worse, its own
     * optimistic `apply` would then write that stale value back on the next drag of that row.
     *
     * Skipped while a drag is live or a write is still pending, because those *are* our own
     * changes arriving back: reloading then would yank the value out from under the pointer.
     */
    let unlisten: UnlistenFn | null = null;

    onMount(() => {
        void load();
        void getCurrentWebviewWindow()
            .listen("player_gain_store_updated", () => {
                if (holding !== null || Object.keys(pendingGain).length > 0) return;
                void load();
            })
            .then((off) => (unlisten = off));
    });

    // A level set in the last moment before the pane closes is still a level the user set.
    onDestroy(() => {
        gainWrites.cancel();
        void flushGain();
        unlisten?.();
    });

    /**
     * Applies a change locally before the command answers.
     *
     * The store is authoritative, but a slider that waits for a round trip fights the thumb
     * under the user's finger. A command that fails reloads, so the optimistic row can never
     * outlive the write it was predicting.
     */
    function apply(cn: string, change: Partial<Pick<PlayerRow, "gain" | "muted">>): void {
        all = all.map((row) => {
            if (row.cn !== cn) return row;
            const gain = change.gain ?? row.gain;
            const muted = change.muted ?? row.muted;
            return {
                ...row,
                gain,
                muted,
                adjusted: PlayersView.isAdjusted({ gain, muted }),
                readout: muted ? "muted" : `${Math.round(gain * 100)}%`,
            };
        });
    }

    /** Runs a command, putting the list back the way the store has it if it fails. */
    async function commit(command: string, args: Record<string, unknown>): Promise<void> {
        try {
            await invoke(command, args);
        } catch {
            await load();
        }
    }

    async function setMuted(row: PlayerRow, muted: boolean): Promise<void> {
        apply(row.cn, { muted });
        await commit("player_settings_set_muted", { cn: row.cn, muted });
    }

    /**
     * Gain writes, coalesced.
     *
     * A drag fires `oninput` per pixel, and each command is a full rewrite of every row in
     * redb plus a mixer feed and an event that makes the dashboard behind this Cover re-fetch.
     * The dashboard's identical slider already routes through a `Coalescer` for exactly this
     * reason. Without it the commands also race: they are `async`, so the last one to *land*
     * wins rather than the last one the user chose.
     */
    let pendingGain: Record<string, number> = {};
    const gainWrites = new Coalescer(120, () => flushGain());

    async function flushGain(): Promise<void> {
        const pending = pendingGain;
        pendingGain = {};
        for (const [cn, gain] of Object.entries(pending)) {
            await commit("player_settings_set_gain", { cn, gain });
        }
    }

    function setGain(row: PlayerRow, gain: number): void {
        apply(row.cn, { gain });
        pendingGain[row.cn] = gain;
        gainWrites.request();
    }

    /** Releases the drag pin and writes the final value without waiting out the coalescer. */
    function endGain(): void {
        holding = null;
        gainWrites.cancel();
        void flushGain();
    }

    async function forget(row: PlayerRow): Promise<void> {
        all = all.filter((entry) => entry.cn !== row.cn);
        await commit("player_settings_forget", { cn: row.cn });
    }

    async function resetAll(): Promise<void> {
        resetting = false;
        // The modal is already closed by this point, so an unhandled rejection here would
        // leave the user looking at an unchanged list with no indication anything went wrong.
        // `load` runs either way: on success it picks up the reset, on failure it puts the
        // list back to whatever the store actually holds.
        try {
            await invoke("player_settings_reset_all");
        } catch (e) {
            error(`PlayersPane: could not reset player settings: ${e}`);
        }
        await load();
    }

    function goToPage(next: number): void {
        page = Math.min(Math.max(0, next), pages - 1);
    }

    // A narrower filter can leave the current page past the end of the list. Clamping here
    // rather than in the click handler covers the search box and the segment too.
    $effect(() => {
        if (page > pages - 1) page = pages - 1;
    });
</script>

<div class="rad-section">
    <div class="rad-section__note">
        {I18n.t("Turn someone down, or mute them, and it stays that way after they walk off. Only you hear the difference.")}
    </div>

    <div class="rad-swatchrow" style="margin-bottom: 4px">
        <Segmented
            label={I18n.t("Which players")}
            value={scope}
            options={[
                { value: "adjusted", label: "Adjusted" },
                { value: "all", label: "Everyone" },
            ]}
            onchange={(next) => {
                scope = next as PlayerScope;
                page = 0;
            }}
        />
        <span class="rad-search" style="flex: 1 1 200px">
            <Icon name="search" />
            <input
                type="search"
                placeholder={I18n.t("Search players")}
                aria-label={I18n.t("Search players")}
                bind:value={query}
                oninput={() => (page = 0)}
            />
        </span>
        <StatusChip>{meta}</StatusChip>
    </div>

    <ListShell
        state={listState}
        count={matching.length}
        failTitle="Could not read your player settings"
        failNote={failure}
        emptyTitle={empty.title}
        emptyNote={empty.note}
        onretry={() => void load()}
    >
        <div class="rad-card">
            <div style="padding: 4px 16px 10px">
                {#each shown as row (row.cn)}
                    <div class="rad-recent-row" data-muted={row.muted ? "true" : undefined}>
                        <span class="rad-server-id rad-recent-row__id">
                            <ServerGlyph name={row.cn} size={34} />
                        </span>
                        <span class="rad-recent-row__text">
                            <span class="rad-recent-row__name">{row.name}</span>
                            <span class="rad-recent-row__seen">{row.seen}</span>
                        </span>
                        <button
                            class="rad-player__mute"
                            aria-pressed={row.muted}
                            aria-label="{row.muted ? 'Unmute' : 'Mute'} {row.name}"
                            onclick={() => void setMuted(row, !row.muted)}
                        >
                            <Icon name={row.muted ? "micoff" : "mic"} />
                        </button>
                        <input
                            class="rad-range"
                            type="range"
                            min="0"
                            max="1.5"
                            step="0.05"
                            value={row.gain}
                            disabled={row.muted}
                            aria-label={I18n.tf("Volume for {name}", { name: row.name })}
                            onpointerdown={() => (holding = row.cn)}
                            oninput={(e) => setGain(row, Number(e.currentTarget.value))}
                            onpointerup={endGain}
                            onpointercancel={endGain}
                            onblur={endGain}
                        />
                        <span class="rad-player__percent">{row.readout}</span>
                        <button
                            class="rad-icon-btn"
                            aria-label={I18n.tf("Forget {name}", { name: row.name })}
                            onclick={() => void forget(row)}
                        >
                            <Icon name="trash" />
                        </button>
                    </div>
                {/each}
            </div>

            {#if pages > 1}
                <div class="rad-pager">
                    <span>{matching.length} shown</span>
                    <span class="rad-pager__pages">
                        <button
                            disabled={page === 0}
                            aria-label={I18n.t("Previous page")}
                            onclick={() => goToPage(page - 1)}>‹</button
                        >
                        {#each pageSlots as slot, index (index)}
                            {#if slot === null}
                                <span aria-hidden="true">…</span>
                            {:else}
                                <button
                                    class={slot === page ? "is-on" : ""}
                                    aria-current={slot === page ? "page" : undefined}
                                    aria-label={I18n.tf("Page {page}", { page: slot + 1 })}
                                    onclick={() => goToPage(slot)}
                                >
                                    {slot + 1}
                                </button>
                            {/if}
                        {/each}
                        <button
                            disabled={page === pages - 1}
                            aria-label={I18n.t("Next page")}
                            onclick={() => goToPage(page + 1)}
                        >
                            ›
                        </button>
                    </span>
                </div>
            {/if}
        </div>
    </ListShell>

    <div class="rad-card">
        <div class="rad-row">
            <span class="rad-row__text">
                <span class="rad-row__label">{I18n.t("Reset everybody")}</span>
                <span class="rad-row__note">
                    {I18n.t("Puts every player back to full volume and unmutes them all. Use this if somebody is silent and you do not remember why.")}
                </span>
            </span>
            <span class="rad-row__control">
                <button
                    class="rad-btn rad-btn--danger"
                    disabled={adjustedCount === 0}
                    onclick={() => (resetting = true)}
                >
                    {I18n.t("Reset everybody…")}
                </button>
            </span>
        </div>
    </div>
</div>

{#if resetting}
    <div class="rad-scrim rad-scrim--modal is-on"></div>
    <div class="rad-modal is-open">
        <h5 class="rad-modal__title">{I18n.t("Reset everybody?")}</h5>
        <p>
            <b>{adjustedCount} player{adjustedCount === 1 ? "" : "s"}</b> on this server go back
            to full volume, unmuted. Other servers are not affected, and nobody leaves the list.
        </p>
        <div class="rad-modal__actions">
            <button class="rad-btn" onclick={() => (resetting = false)}>{I18n.t("Cancel")}</button>
            <button class="rad-btn rad-btn--danger" onclick={() => void resetAll()}>{I18n.t("Reset")}</button>
        </div>
    </div>
{/if}
