import type { Game } from "../../bindings/Game";
import type { Permission } from "../../bindings/Permission";

/** Banned is a state of its own, not a flag on top of online or offline. */
export type ManagedPlayerStatus = "banned" | "online" | "offline";

export interface ManagedPlayerRow {
    /** `game:gamertag`. The row key, because a gamertag alone is not unique across games. */
    readonly key: string;
    readonly gamertag: string;
    readonly game: Game;
    readonly status: ManagedPlayerStatus;
    readonly banned: boolean;
    /** The effective set, for badging. The editor fetches the overrides separately. */
    readonly permissions: readonly Permission[];
    readonly added: string;
}

/**
 * One block in a row's status strip.
 *
 * Colour is the whole visual, so the label is not decoration: it is what names the block
 * for a screen reader and for anybody who cannot separate two hues.
 */
export interface RosterBlock {
    readonly color: string;
    readonly label: string;
    /** False for a slot held open so the strip stays positional down a column of rows. */
    readonly on: boolean;
}
