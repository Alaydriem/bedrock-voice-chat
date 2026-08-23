export interface PlateChip {
    readonly label: string;
    readonly severity: "idle" | "ok" | "warn" | "bad" | "muted";
}

/** One place you can point at. Shared by Proxy and Realms. */
export interface Plate {
    readonly id: string;
    readonly name: string;
    /** The mono line under the name: an address, or a Realm's message of the day. */
    readonly detail: string;
    /** Seeds the derived artwork and the glyph. */
    readonly glyphKey: string;
    readonly chips: readonly PlateChip[];
    readonly favourite: boolean;
    readonly active: boolean;
    /** False for a Realm that is closed, or a backend the server will not accept. */
    readonly reachable: boolean;
    /** Operator-supplied entries are theirs: not editable, not removable here. */
    readonly readonly: boolean;
}
