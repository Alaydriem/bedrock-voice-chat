import type { RingMode } from '$radial/bindings/RingBinding';
import type { ServerRosterEntry } from './ServerRosterEntry';

export interface RosterRowViewState {
    /** Chip severity. `idle` is a check still running, not a good result. */
    readonly severity: 'idle' | 'ok' | 'warn' | 'bad';
    /** The chip, in a couple of words. */
    readonly status: string;
    /** The primary action's label. Empty when there is nothing to offer. */
    readonly action: string;
    /** Why the primary action is unavailable, or empty when it is available. */
    readonly blocked: string;
    /** What the ring shows while this row is the one being considered. */
    readonly ring: RingMode;
    readonly caption: string;
}

/**
 * What one row says, from its status alone.
 *
 * The chip and the ring answer different questions and are allowed to disagree: the ring
 * is reachability, so a server whose sign-in has lapsed still reads as found, while the
 * chip is what the person has to do about it. Collapsing them would lose the distinction
 * between "that server is down" and "your sign-in expired", which is the difference
 * between waiting for an operator and pressing a button.
 */
export class RosterRowView {
    static of(entry: ServerRosterEntry): RosterRowViewState {
        switch (entry.status) {
            case 'checking':
                return {
                    severity: 'idle',
                    status: 'Checking',
                    action: '',
                    blocked: '',
                    ring: 'empty',
                    caption: 'CHECKING',
                };

            case 'connect':
                return {
                    severity: 'ok',
                    status: 'Ready',
                    action: 'Join',
                    blocked: '',
                    ring: 'lock',
                    caption: 'READY TO JOIN',
                };

            case 'reauth':
                return {
                    severity: 'warn',
                    status: 'Sign in again',
                    action: 'Sign in',
                    blocked: '',
                    ring: 'lock',
                    caption: 'SIGN-IN EXPIRED',
                };

            case 'missing':
                return {
                    severity: 'warn',
                    status: 'No sign-in saved',
                    action: 'Sign in',
                    blocked: '',
                    ring: 'empty',
                    caption: 'NOT SIGNED IN',
                };

            case 'version_mismatch':
                return entry.clientTooOld
                    ? {
                          severity: 'warn',
                          status: 'Update needed',
                          action: 'Update',
                          blocked: '',
                          ring: 'lock',
                          caption: `SERVER ON ${entry.serverVersion || '—'}`,
                      }
                    : {
                          severity: 'warn',
                          status: 'Server is older',
                          action: '',
                          blocked: `That server speaks ${entry.serverVersion || 'an older protocol'} and this app speaks ${entry.clientVersion || 'a newer one'}. Whoever runs it has to update it.`,
                          ring: 'lock',
                          caption: `SERVER ON ${entry.serverVersion || '—'}`,
                      };

            case 'unreachable':
                return {
                    severity: 'bad',
                    status: 'Not answering',
                    // Retrying is the only useful thing to offer: a server that is down is
                    // not fixed by signing in, and this app cannot fix it either.
                    action: 'Try again',
                    blocked: '',
                    ring: 'empty',
                    caption: 'NO RESPONSE',
                };
        }
    }

    /** Whether choosing this row leads to the dashboard rather than anywhere else. */
    static isJoinable(entry: ServerRosterEntry): boolean {
        return entry.status === 'connect';
    }

    /**
     * Which row the ring should read while nothing is being considered.
     *
     * The one that can be joined, so the pane rests on the answer rather than on whichever
     * server happens to be stored first. Falls back to the head of the list, so a list of
     * entirely broken servers still resolves to something.
     */
    static resting(entries: readonly ServerRosterEntry[]): ServerRosterEntry | null {
        if (entries.length === 0) return null;
        return (
            entries.find((entry) => entry.isCurrent && RosterRowView.isJoinable(entry)) ??
            entries.find(RosterRowView.isJoinable) ??
            entries[0]
        );
    }
}
