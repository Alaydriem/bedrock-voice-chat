import { I18n } from "$lib/i18n";
import type { ExportOutcome } from "../../bindings/ExportOutcome";
import type { RecordingTrack } from "../../bindings/RecordingTrack";
import type { TrackGroup } from "./TrackGroup";

/** A session's tracks, as the session screen shows them. */
export class RecordingTracksView {
    /** A heading appears only over a group that has something in it. */
    static groups(tracks: readonly RecordingTrack[]): readonly TrackGroup[] {
        const of = (kind: RecordingTrack["kind"]) => tracks.filter((t) => t.kind === kind);
        return [
            { heading: null, tracks: of("Own") },
            { heading: I18n.t("players"), tracks: of("Player") },
            { heading: null, tracks: of("Jukebox") },
        ].filter((group) => group.tracks.length > 0);
    }

    /** The command wants keys; a checkbox is labelled with a display name. */
    static keysFor(
        tracks: readonly RecordingTrack[],
        chosen: ReadonlySet<string>,
    ): readonly string[] {
        return tracks.filter((t) => chosen.has(t.display)).flatMap((t) => t.keys);
    }

    /** The tracks a set of chosen names stands for, in the order the session lists them. */
    static chosenTracks(
        tracks: readonly RecordingTrack[],
        chosen: ReadonlySet<string>,
    ): readonly RecordingTrack[] {
        return tracks.filter((t) => chosen.has(t.display));
    }

    /** How many things play into one jukebox track. A voice is always one thing. */
    static sourceNote(track: RecordingTrack): string {
        if (track.kind !== "Jukebox") return "";
        return I18n.tf("{count} source{plural}", {
            count: String(track.keys.length),
            plural: track.keys.length === 1 ? "" : "s",
        });
    }

    static summary(outcome: ExportOutcome): string {
        const total = outcome.written.length + outcome.failed.length;
        if (outcome.failed.length === 0) {
            return I18n.tf("{count} track{plural} written", {
                count: String(total),
                plural: total === 1 ? "" : "s",
            });
        }
        return I18n.tf("{written} of {total} written — {names} failed", {
            written: String(outcome.written.length),
            total: String(total),
            names: outcome.failed.map((f) => f.track).join(", "),
        });
    }
}
