import type { PreflightStep } from './preflight/PreflightStep';
import type { RosterStatus } from './RosterStatus';

/** One saved server, as a plate draws it. */
export interface ServerRosterEntry {
    /** The stored URL, which is what every command keys on. */
    readonly server: string;
    /** The URL without its scheme, which is what a person recognises. */
    readonly host: string;
    readonly player: string;
    readonly game: string;
    readonly status: RosterStatus;
    readonly steps: readonly PreflightStep[];

    /** The handshake's round trip, once one has been measured. */
    readonly rtt: number;
    readonly slow: boolean;
    readonly quicPort: number;
    readonly serverVersion: string;
    readonly clientVersion: string;
    readonly clientTooOld: boolean;

    /**
     * Operator art, absent far more often than not. `avatar.png` takes the identity tile
     * and `canvas.png` fills the head of the plate; empty means fall back to the glyph and
     * the derived hue, which is the case that always works.
     */
    readonly avatarUrl: string;
    readonly canvasUrl: string;

    /**
     * Something that happened to this plate rather than something about the server — an
     * update that turned out not to exist, a removal that failed. Cleared by rechecking.
     */
    readonly note?: string;
}
