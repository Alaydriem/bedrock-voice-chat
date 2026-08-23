import type { RecordingTrack } from "../../bindings/RecordingTrack";

export interface TrackGroup {
    /** Null where the group needs no rule over it, which is where you are. */
    readonly heading: string | null;
    readonly tracks: readonly RecordingTrack[];
}
