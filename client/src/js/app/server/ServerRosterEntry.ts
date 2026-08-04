import type { RosterStatus } from './RosterStatus';

/** One saved server, as the list draws it. */
export interface ServerRosterEntry {
    /** The stored URL, which is what every command keys on. */
    readonly server: string;
    /** The URL without its scheme, which is what a person recognises. */
    readonly host: string;
    readonly player: string;
    readonly game: string;
    readonly status: RosterStatus;
    readonly serverVersion: string;
    readonly clientVersion: string;
    readonly clientTooOld: boolean;
    /** The server this app was last signed in to, which the list ticks. */
    readonly isCurrent: boolean;
    /**
     * Something that happened to this row rather than something about the server — an
     * update that turned out not to exist, a removal that failed. Cleared by re-checking.
     */
    readonly note?: string;
}
