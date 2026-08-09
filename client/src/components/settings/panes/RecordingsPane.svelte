<script lang="ts">
  import { I18n } from "$lib/i18n";
    import { invoke } from "@tauri-apps/api/core";
    import { onMount } from "svelte";
    import Icon from "$radial/components/Icon.svelte";
    import StatusChip from "$radial/components/StatusChip.svelte";
    import Toggle from "$radial/components/Toggle.svelte";
    import type { RecordingSession } from "../../../js/bindings/RecordingSession";
    import type { VoiceRuntimeState } from "../../../js/bindings/VoiceRuntimeState";
    import type { RecordingRow } from "../../../js/app/settings/RecordingRow";
    import { RecordingsView } from "../../../js/app/settings/RecordingsView";
    import type { ListState } from "../../../js/app/settings/ListState";
    import ListShell from "../ListShell.svelte";

    let listState = $state<ListState>("loading");
    let recordAllowed = $state(true);
    let rows = $state<readonly RecordingRow[]>([]);
    let failure = $state("");

    /** The row whose menu is open, and the dialog it asked for. */
    let renaming = $state<RecordingRow | null>(null);
    let renameTo = $state("");
    let exporting = $state<RecordingRow | null>(null);
    let chosen = $state<ReadonlySet<string>>(new Set());
    let spatial = $state(true);
    let deleting = $state<RecordingRow | null>(null);

    const total = $derived(RecordingsView.totalSize(rows));

    async function load(): Promise<void> {
        listState = "loading";
        try {
            rows = RecordingsView.rows(
                await invoke<RecordingSession[]>("get_recording_sessions"),
            );
            listState = "ready";
        } catch (e) {
            failure = e instanceof Error ? e.message : String(e);
            listState = "failed";
        }
    }

    // Permissive on a failed read, the same answer an unasked server gives. A transient
    // error must never invent a policy the operator did not state.
    async function loadPolicy(): Promise<void> {
        try {
            const backend = await invoke<VoiceRuntimeState>("voice_runtime_state");
            recordAllowed = backend.recordingAllowed;
        } catch {
            recordAllowed = true;
        }
    }

    onMount(() => {
        void load();
        void loadPolicy();
    });

    function openExport(row: RecordingRow): void {
        exporting = row;
        chosen = new Set(row.players);
        spatial = true;
    }

    function toggleTrack(name: string): void {
        const next = new Set(chosen);
        if (next.has(name)) next.delete(name);
        else next.add(name);
        chosen = next;
    }

    async function runExport(): Promise<void> {
        if (!exporting) return;
        const sessionId = exporting.id;
        exporting = null;
        await invoke("export_recording", {
            sessionId,
            selectedPlayers: [...chosen],
            spatial,
            format: "Mp4Opus",
        }).catch(() => {});
    }

    async function runRename(): Promise<void> {
        if (!renaming) return;
        const sessionId = renaming.id;
        const name = renameTo.trim();
        renaming = null;
        await invoke("rename_recording_session", { sessionId, name }).catch(() => {});
        await load();
    }

    async function runDelete(): Promise<void> {
        if (!deleting) return;
        const sessionId = deleting.id;
        deleting = null;
        await invoke("delete_recording_session", { sessionId }).catch(() => {});
        await load();
    }
</script>

<div class="rad-section">
    <div class="rad-section__note">
        {I18n.t("Every session is stored as one track per player, timecoded together. Export writes the mix and the tracks you pick into the session's own folder.")}
    </div>

    <!-- A callout, not the list's failure state: recordings already on disk stay listed,
         playable and exportable. Only new ones are barred. -->
    {#if !recordAllowed}
        <div class="rad-callout rad-callout--warn">
            <span>
                <b>{I18n.t("This server does not allow recording")}</b>
                {I18n.t("The operator has turned voice recording off, so the record button is unavailable while you are connected here.")}
            </span>
        </div>
    {/if}

    {#if listState === "ready" && rows.length > 0}
        <div class="rad-swatchrow" style="margin-bottom: 4px">
            <StatusChip>
                {rows.length} session{rows.length === 1 ? "" : "s"} · {total}
            </StatusChip>
        </div>
    {/if}

    <ListShell
        state={listState}
        count={rows.length}
        failTitle="Couldn't read your recordings"
        failNote={failure || "The recordings folder could not be read."}
        onretry={() => void load()}
        emptyTitle="Nothing recorded yet"
        emptyNote="Arm recording from the controls on the dashboard. Each player lands on their own timecoded track."
    >
        <div class="rad-card">
            <div class="rad-table-wrap">
                <table class="rad-table">
                    <thead>
                        <tr>
                            <th>{I18n.t("Session")}</th>
                            <th class="rad-num">{I18n.t("Recorded")}</th>
                            <th class="rad-num">{I18n.t("Length")}</th>
                            <th class="rad-num">{I18n.t("Tracks")}</th>
                            <th class="rad-num">{I18n.t("Size")}</th>
                            <th></th>
                        </tr>
                    </thead>
                    <tbody>
                        {#each rows as row (row.id)}
                            <tr>
                                <td>
                                    <span class="rad-table__name">{row.name}</span>
                                    {#if !row.exportable}
                                        <!-- Still being written, or written by a build
                                             whose format this one cannot read. Either
                                             way it can be named and deleted, not
                                             exported. -->
                                        <StatusChip severity="muted">{I18n.t("Not exportable")}</StatusChip>
                                    {/if}
                                </td>
                                <td class="rad-num">{row.recorded}</td>
                                <td class="rad-num">{row.length}</td>
                                <td class="rad-num">{row.tracks}</td>
                                <td class="rad-num">{row.size}</td>
                                <td class="rad-table__actions">
                                    <span class="rad-row-actions">
                                        <button
                                            class="rad-kebab"
                                            disabled={!row.exportable}
                                            onclick={() => openExport(row)}
                                            aria-label={I18n.tf("Export {name}", { name: row.name })}
                                        >
                                            <Icon name="download" />
                                        </button>
                                        <button
                                            class="rad-kebab"
                                            onclick={() => {
                                                renaming = row;
                                                renameTo = row.unnamed ? "" : row.name;
                                            }}
                                            aria-label={I18n.tf("Rename {name}", { name: row.name })}
                                        >
                                            <Icon name="field" />
                                        </button>
                                        <button
                                            class="rad-kebab"
                                            onclick={() => (deleting = row)}
                                            aria-label={I18n.tf("Delete {name}", { name: row.name })}
                                        >
                                            <Icon name="trash" />
                                        </button>
                                    </span>
                                </td>
                            </tr>
                        {/each}
                    </tbody>
                </table>
            </div>
        </div>
    </ListShell>

    <div class="rad-callout">
        <span>
            {I18n.t("An export is written beside the recording it came from, in that session's own folder. Nothing is uploaded —")} <b>a recording never leaves this machine unless you move it
            yourself.</b>
        </span>
    </div>
</div>

{#if exporting}
    <div class="rad-scrim rad-scrim--modal is-on"></div>
    <div class="rad-modal rad-modal--wide is-open">
        <h5 class="rad-modal__title">{I18n.t("Export this recording")}</h5>
        <p>{I18n.t("Pick who to include. Each player is written as their own track alongside the mix.")}</p>
        <div class="rad-card" style="margin-top: 14px">
            <div class="rad-modal__scroll">
                {#each exporting.players as player (player)}
                    <button
                        class="rad-checkbox"
                        role="checkbox"
                        aria-checked={chosen.has(player)}
                        onclick={() => toggleTrack(player)}
                    >
                        <span class="rad-checkbox__box"><Icon name="check" /></span>
                        <span class="rad-checkbox__label">{player}</span>
                    </button>
                {/each}
            </div>
        </div>
        <div class="rad-card" style="margin-top: 12px">
            <div class="rad-row">
                <span class="rad-row__text">
                    <span class="rad-row__label">{I18n.t("Mix in the spatial positions")}</span>
                    <span class="rad-row__note">
                        {I18n.t("Places each voice where it was standing. Off writes every track flat and centred, which is what you want if you are going to mix it yourself.")}
                    </span>
                </span>
                <span class="rad-row__control">
                    <Toggle
                        checked={spatial}
                        label={I18n.t("Mix in the spatial positions")}
                        onchange={(v) => (spatial = v)}
                    />
                </span>
            </div>
        </div>
        <div class="rad-modal__actions">
            <button class="rad-btn" onclick={() => (exporting = null)}>{I18n.t("Cancel")}</button>
            <button
                class="rad-btn rad-btn--primary"
                disabled={chosen.size === 0}
                onclick={() => void runExport()}
            >
                Export {chosen.size} track{chosen.size === 1 ? "" : "s"}
            </button>
        </div>
    </div>
{/if}

{#if renaming}
    <div class="rad-scrim rad-scrim--modal is-on"></div>
    <div class="rad-modal is-open">
        <h5 class="rad-modal__title">{I18n.t("Rename this recording")}</h5>
        <p>{I18n.t("Only the name changes. The recording and its tracks stay where they are.")}</p>
        <span class="rad-input" style="margin-top: 12px; width: 100%">
            <!-- svelte-ignore a11y_autofocus -->
            <input
                type="text"
                bind:value={renameTo}
                placeholder={renaming.recorded}
                aria-label={I18n.t("Recording name")}
                autofocus
            />
        </span>
        <div class="rad-modal__actions">
            <button class="rad-btn" onclick={() => (renaming = null)}>{I18n.t("Cancel")}</button>
            <button class="rad-btn rad-btn--primary" onclick={() => void runRename()}>{I18n.t("Rename")}</button>
        </div>
    </div>
{/if}

{#if deleting}
    <div class="rad-scrim rad-scrim--modal is-on"></div>
    <div class="rad-modal is-open">
        <h5 class="rad-modal__title">{I18n.t("Delete this recording?")}</h5>
        <p>
            <b>{deleting.name}</b> and all of its tracks will be removed from disk. This cannot be
            undone.
        </p>
        <div class="rad-modal__actions">
            <button class="rad-btn" onclick={() => (deleting = null)}>{I18n.t("Keep it")}</button>
            <button class="rad-btn rad-btn--danger" onclick={() => void runDelete()}>
                <Icon name="trash" /> {I18n.t("Delete")}
            </button>
        </div>
    </div>
{/if}
