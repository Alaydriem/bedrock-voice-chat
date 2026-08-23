import type { RosterStatus } from '../RosterStatus';

/** What a completed preflight concluded, beyond the steps themselves. */
export interface PreflightOutcome {
    readonly status: Exclude<RosterStatus, 'checking'>;
    /** The handshake's round trip in milliseconds, which is the only one measured. */
    readonly rtt: number;
    /** High enough to be worth saying out loud. */
    readonly slow: boolean;
    /** The QUIC port that answered, or the advertised one when none did. */
    readonly quicPort: number;
    readonly serverVersion: string;
    readonly clientVersion: string;
    readonly clientTooOld: boolean;
}
