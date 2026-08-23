<script lang="ts">
    import { I18n } from "$lib/i18n";
    import { invoke } from "@tauri-apps/api/core";
    import { listen, type UnlistenFn } from "@tauri-apps/api/event";
    import { onDestroy, onMount } from "svelte";
    import Icon from "$radial/components/Icon.svelte";
    import StatusChip from "$radial/components/StatusChip.svelte";
    import type { ExportOutcome } from "../../../js/bindings/ExportOutcome";
    import type { ExportProgress } from "../../../js/bindings/ExportProgress";
    import type { RecordingSession } from "../../../js/bindings/RecordingSession";
    import type { RecordingTrack } from "../../../js/bindings/RecordingTrack";
    import type { VoiceRuntimeState } from "../../../js/bindings/VoiceRuntimeState";
    import type { RecordingRow } from "../../../js/app/settings/RecordingRow";
    import { RecordingsView } from "../../../js/app/settings/RecordingsView";
    import { RecordingTracksView } from "../../../js/app/settings/RecordingTracksView";
    import type { ListState } from "../../../js/app/settings/ListState";
    import ListShell from "../ListShell.svelte";
    import RecordingDetail from "../RecordingDetail.svelte";

    let listState = $state<ListState>("loading");
    let recordAllowed = $state(true);
    let rows = $state<readonly RecordingRow[]>([]);
    let failure = $state("");

    /** The session being looked at. Null while the table is showing. */
    let viewing = $state<RecordingRow | null>(null);
    let tracks = $state<readonly RecordingTrack[]>([]);
    let chosen = $state<ReadonlySet<string>>(new Set());

    /**
     * Off by default: a flat track is the recording as it was captured, with nothing
     * re-encoded. Turning it on is the choice to place each voice where it was standing.
     */
    let spatial = $state(false);

    /**
     * A run belongs to the session, not to the screen that started it, so leaving mid-run
     * is allowed and the row reports it.
     */
    let running = $state<{ id: string; track: string; index: number; total: number } | null>(null);
    let report = $state<{ id: string; text: string; failed: boolean } | null>(null);

    /** The row whose menu is open, and the dialog it asked for. */
    let renaming = $state<RecordingRow | null>(null);
    let renameTo = $state("");
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

    let unlisten: UnlistenFn | null = null;

    onMount(() => {
        void load();
        void loadPolicy();
        void (async () => {
            unlisten = await listen<ExportProgress>("recording:export-progress", (event) => {
                const at = event.payload;
                if (running?.id !== at.session_id) return;
                running = {
                    id: at.session_id,
                    track: at.track,
                    index: at.index,
                    total: at.total,
                };
            });
        })();
    });

    onDestroy(() => unlisten?.());

    async function open(row: RecordingRow): Promise<void> {
        viewing = row;
        tracks = await invoke<RecordingTrack[]>("get_recording_tracks", {
            sessionId: row.id,
        }).catch(() => []);
        chosen = new Set(tracks.map((track) => track.display));
    }

    function toggleTrack(display: string): void {
        const next = new Set(chosen);
        if (next.has(display)) next.delete(display);
        else next.add(display);
        chosen = next;
    }

    async function runExport(): Promise<void> {
        if (!viewing) return;
        const id = viewing.id;
        const picked = RecordingTracksView.chosenTracks(tracks, chosen);
        if (picked.length === 0) return;

        running = { id, track: picked[0].display, index: 0, total: picked.length };
        report = null;

        const outcome = await invoke<ExportOutcome>("export_recording", {
            sessionId: id,
            tracks: picked,
            spatial,
            format: "Mp4Opus",
        }).catch(() => null);

        running = null;
        report = outcome
            ? {
                  id,
                  text: RecordingTracksView.summary(outcome),
                  failed: outcome.failed.length > 0,
              }
            : { id, text: I18n.t("The export could not run."), failed: true };
    }

    async function runRename(): Promise<void> {
        if (!renaming) return;
        const sessionId = renaming.id;
        const name = renameTo.trim();
        renaming = null;
        await invoke("rename_recording_session", { sessionId, name }).catch(() => {});
        await load();
        if (viewing?.id === sessionId) {
            viewing = rows.find((row) => row.id === sessionId) ?? null;
        }
    }

    async function runDelete(): Promise<void> {
        if (!deleting) return;
        const sessionId = deleting.id;
        deleting = null;
        await invoke("delete_recording_session", { sessionId }).catch(() => {});
        // The screen you are standing on is about to stop existing.
        if (viewing?.id === sessionId) viewing = null;
        await load();
    }
</script>

{#if viewing}
    <RecordingDetail
        row={viewing}
        {tracks}
        {chosen}
        progress={running?.id === viewing.id ? running : null}
        status={report?.id === viewing.id ? report.text : ""}
        failed={report?.id === viewing.id && report.failed}
        onback={() => (viewing = null)}
        ontoggle={toggleTrack}
        onall={() => (chosen = new Set(tracks.map((track) => track.display)))}
        onnone={() => (chosen = new Set())}
        {spatial}
        onspatial={(value) => (spatial = value)}
        onexport={() => void runExport()}
        onrename={() => {
            renaming = viewing;
            renameTo = viewing?.unnamed ? "" : (viewing?.name ?? "");
        }}
        ondelete={() => (deleting = viewing)}
    />
{:else}
    <div class="rad-section">
        <div class="rad-section__note">
            {I18n.t(
                "Export your sessions to disk to import into your DAW or video editor. Players are exported to their own timecode encoded audio track.",
            )}
        </div>

        <!-- A callout, not the list's failure state: recordings already on disk stay listed,
             playable and exportable. Only new ones are barred. -->
        {#if !recordAllowed}
            <div class="rad-callout rad-callout--warn">
                <span>
                    <b>{I18n.t("This server does not allow recording")}</b>
                    {I18n.t(
                        "The operator has turned voice recording off, so the record button is unavailable while you are connected here.",
                    )}
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
                                <th class="rad-num">{I18n.t("Players")}</th>
                                <th class="rad-num">{I18n.t("Size")}</th>
                                <th></th>
                            </tr>
                        </thead>
                        <tbody>
                            {#each rows as row (row.id)}
                                <tr
                                    class="is-openable"
                                    onclick={() => void open(row)}
                                    aria-label={I18n.tf("Open {name}", { name: row.name })}
                                >
                                    <td>
                                        <span class="rad-table__name">{row.name}</span>
                                        {#if !row.exportable}
                                            <!-- Still being written, or written by a build
                                                 whose format this one cannot read. Either
                                                 way it can be named and deleted, not
                                                 exported. -->
                                            <StatusChip severity="muted">
                                                {I18n.t("Not exportable")}
                                            </StatusChip>
                                        {/if}
                                        {#if running?.id === row.id}
                                            <StatusChip severity="warn">
                                                {I18n.tf("Exporting {index}/{total}", {
                                                    index: String(running.index),
                                                    total: String(running.total),
                                                })}
                                            </StatusChip>
                                        {/if}
                                    </td>
                                    <td class="rad-num">{row.recorded}</td>
                                    <td class="rad-num">{row.length}</td>
                                    <td class="rad-num">{row.players}</td>
                                    <td class="rad-num">{row.size}</td>
                                    <td class="rad-table__actions">
                                        <span class="rad-table__go"><Icon name="chev" /></span>
                                    </td>
                                </tr>
                            {/each}
                        </tbody>
                    </table>
                </div>
            </div>
        </ListShell>
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
            <button class="rad-btn rad-btn--primary" onclick={() => void runRename()}>
                {I18n.t("Rename")}
            </button>
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
