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
    readonly players: readonly string[];
    readonly tracks: number;
    readonly size: string;
    readonly bytes: number;
    /** False while the session is still being written, or written by an older build. */
    readonly exportable: boolean;
}
