export interface RecordingRow {
    readonly id: string;
    /** The given name, or when it was recorded. Never blank. */
    readonly name: string;
    /** True when `name` is the recorded time standing in for an absent name. */
    readonly unnamed: boolean;
    readonly recorded: string;
    /** Sortable, unlike `recorded`. */
    readonly recordedAt: number;
    readonly length: string;
    /**
     * How many people the session received audio from. Not its track count: your own
     * voice and the jukebox are exportable tracks this number never included, and finding
     * the real total means reading the session's own directory.
     */
    readonly players: number;
    readonly size: string;
    readonly bytes: number;
    /** False while the session is still being written, or written by an older build. */
    readonly exportable: boolean;
}
