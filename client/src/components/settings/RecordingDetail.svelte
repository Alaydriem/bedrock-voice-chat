<script lang="ts">
    import { I18n } from "$lib/i18n";
    import Icon from "$radial/components/Icon.svelte";
    import StatusChip from "$radial/components/StatusChip.svelte";
    import Toggle from "$radial/components/Toggle.svelte";
    import type { RecordingTrack } from "../../js/bindings/RecordingTrack";
    import type { RecordingRow } from "../../js/app/settings/RecordingRow";
    import { RecordingTracksView } from "../../js/app/settings/RecordingTracksView";

    interface Props {
        row: RecordingRow;
        tracks: readonly RecordingTrack[];
        chosen: ReadonlySet<string>;
        /** Set while this session is rendering. Null when it is not. */
        progress: { track: string; index: number; total: number } | null;
        /** The last run's report, or an empty string before the first one. */
        status?: string;
        failed?: boolean;
        onback: () => void;
        ontoggle: (display: string) => void;
        onall: () => void;
        onnone: () => void;
        onexport: () => void;
        onrename: () => void;
        ondelete: () => void;
    }
    let {
        row,
        tracks,
        chosen,
        progress,
        status = "",
        failed = false,
        onback,
        ontoggle,
        onall,
        onnone,
        onexport,
        onrename,
        ondelete,
    }: Props = $props();

    const groups = $derived(RecordingTracksView.groups(tracks));
    const busy = $derived(progress !== null);
</script>

<div class="rad-section">
    <div class="rad-detail__head">
        <button
            class="rad-detail__back"
            data-rec-back
            onclick={onback}
            aria-label={I18n.t("Back to recordings")}
        >
            <Icon name="back" />
        </button>
        <span class="rad-detail__text">
            <span class="rad-detail__title">{row.name}</span>
            <span class="rad-detail__meta">
                {row.length} · {row.size} · {tracks.length} track{tracks.length === 1 ? "" : "s"} · {row.recorded}
            </span>
        </span>
        {#if !row.exportable}
            <StatusChip severity="muted">{I18n.t("Not exportable")}</StatusChip>
        {/if}
    </div>

    <!-- Which of the two reasons applies is the part a disabled button cannot say. -->
    {#if !row.exportable}
        <div class="rad-callout rad-callout--warn" style="margin-top: 16px">
            <span>
                <b>{I18n.t("This recording cannot be exported")}</b>
                {I18n.t(
                    "It is either still being written, or it was written by an older build whose format this version cannot read. It can still be renamed or deleted.",
                )}
            </span>
        </div>
    {/if}

    <div class="rad-tracks__head">
        <span class="rad-label">{I18n.t("Tracks")}</span>
        <span class="rad-spacer"></span>
        <button
            class="rad-btn rad-btn--quiet"
            data-track-all
            onclick={onall}
            disabled={!row.exportable || busy}
        >
            {I18n.t("All")}
        </button>
        <button
            class="rad-btn rad-btn--quiet"
            data-track-none
            onclick={onnone}
            disabled={!row.exportable || busy}
        >
            {I18n.t("None")}
        </button>
    </div>

    <div class="rad-card" style="margin-top: 8px">
        <div class="rad-tracklist">
            {#each groups as group (group.heading ?? group.tracks[0].display)}
                {#if group.heading}
                    <span class="rad-tracklist__group">{group.heading}</span>
                {/if}
                {#each group.tracks as track (track.display)}
                    <button
                        class="rad-checkbox"
                        class:rad-tracklist__wide={track.kind === "Jukebox"}
                        role="checkbox"
                        aria-checked={chosen.has(track.display)}
                        aria-label={track.display}
                        disabled={!row.exportable || busy}
                        onclick={() => ontoggle(track.display)}
                    >
                        <span class="rad-checkbox__box"><Icon name="check" /></span>
                        <span class="rad-checkbox__label">{track.display}</span>
                        {#if track.kind === "Own"}
                            <span class="rad-checkbox__note">{I18n.t("you")}</span>
                        {:else if track.kind === "Jukebox"}
                            <span class="rad-checkbox__note">
                                {RecordingTracksView.sourceNote(track)}
                            </span>
                        {/if}
                    </button>
                {/each}
            {/each}
        </div>
    </div>

    <div class="rad-card">
        <div class="rad-row">
            <span class="rad-row__text">
                <span class="rad-row__label">{I18n.t("Mix in the spatial positions")}</span>
                <span class="rad-row__note">
                    {I18n.t(
                        "Not built yet. Every track is written flat and centred, which is what you want anyway if you are going to mix it yourself.",
                    )}
                </span>
            </span>
            <span class="rad-row__control">
                <Toggle
                    checked={false}
                    disabled
                    label={I18n.t("Mix in the spatial positions")}
                    onchange={() => {}}
                />
            </span>
        </div>
    </div>

    {#if progress}
        <div class="rad-progress" style="margin-top: 16px">
            <i style="width: {Math.round((progress.index / progress.total) * 100)}%"></i>
        </div>
    {/if}

    <div class="rad-detail__foot">
        <button class="rad-btn" onclick={onrename}>{I18n.t("Rename…")}</button>
        <button class="rad-btn rad-btn--danger" onclick={ondelete}>{I18n.t("Delete")}</button>
        <span class="rad-spacer"></span>
        <span class="rad-detail__status" class:rad-detail__status--bad={failed}>
            {progress
                ? I18n.tf("Rendering {track} — {index} of {total}", {
                      track: progress.track,
                      index: String(progress.index),
                      total: String(progress.total),
                  })
                : status}
        </span>
        <button
            class="rad-btn rad-btn--primary"
            data-export-go
            disabled={!row.exportable || chosen.size === 0 || busy}
            onclick={onexport}
        >
            {busy
                ? I18n.t("Exporting…")
                : I18n.tf("Export {count} track{plural}", {
                      count: String(chosen.size),
                      plural: chosen.size === 1 ? "" : "s",
                  })}
        </button>
    </div>
</div>
