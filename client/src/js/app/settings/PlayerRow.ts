/** One player in the Players pane, as the row renders them. */
export interface PlayerRow {
    /** The canonical `game:gamertag` — the key every command takes. */
    readonly cn: string;
    /** The bare gamertag, which is what a human reads. */
    readonly name: string;
    readonly gain: number;
    readonly muted: boolean;
    /** Unix ms, or null for a row written before it was stamped. */
    readonly lastSeen: number | null;
    /** Whether the user has decided anything about this player. */
    readonly adjusted: boolean;
    /** How long ago they were nearby, in words. */
    readonly seen: string;
    /** The volume readout: a percentage, or `muted`. */
    readonly readout: string;
}
