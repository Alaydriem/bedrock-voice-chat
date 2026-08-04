import type { ServerRosterEntry } from '../ServerRosterEntry';
import type { PreflightStep } from './PreflightStep';

export type VerdictSeverity = 'ok' | 'warn' | 'bad';

export interface Verdict {
    readonly severity: VerdictSeverity;
    readonly sentence: string;
}

/**
 * The preflight in one sentence, first failing check wins.
 *
 * Somebody reading this has a problem. A summary that averages four results into "mostly
 * fine" leaves them to find the one line that is not, and a wall of green with one red
 * line in the middle makes them do the diagnosis themselves.
 */
export class PreflightVerdict {
    static of(entry: ServerRosterEntry): Verdict {
        if (entry.status === 'checking') {
            return { severity: 'ok', sentence: 'Checking…' };
        }

        const failed = entry.steps.find((step) => step.state === 'bad');
        if (failed) return PreflightVerdict.forFailure(failed, entry);

        if (entry.steps.some((step) => step.state === 'warn')) {
            return {
                severity: 'warn',
                sentence: `${entry.rtt} ms to this server. Voice works, with delay you will notice.`,
            };
        }

        return { severity: 'ok', sentence: 'Everything looks fine.' };
    }

    private static forFailure(failed: PreflightStep, entry: ServerRosterEntry): Verdict {
        switch (failed.name) {
            case 'Credentials':
                return {
                    severity: 'bad',
                    sentence:
                        'You are not signed in to this server any more. Sign in again to connect.',
                };

            case 'Handshake':
                return {
                    severity: 'bad',
                    sentence:
                        entry.status === 'reauth'
                            ? 'This server refused your saved sign-in. Sign in again to connect.'
                            : 'This server is not answering. Nothing is wrong on this device — ask whoever runs it.',
                };

            case 'Protocol':
                return {
                    severity: 'bad',
                    sentence: entry.clientTooOld
                        ? `This server speaks protocol ${entry.serverVersion} and your client speaks ${entry.clientVersion}. Update the client to connect.`
                        : `This server speaks protocol ${entry.serverVersion} and your client speaks ${entry.clientVersion}. Whoever runs it has to update it.`,
                };

            case 'QUIC path':
                return {
                    severity: 'bad',
                    sentence:
                        `UDP ${entry.quicPort} to this server is blocked, so voice cannot connect at all. ` +
                        'Check the network or firewall you are behind, then recheck.',
                };
        }
    }
}
