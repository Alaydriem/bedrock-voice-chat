import type { PlateAction } from './PlateAction';
import type { ServerRosterEntry } from './ServerRosterEntry';
import type { TallyItem } from './TallyItem';

export interface PlateViewState {
    /** State in one word. `muted` is a check still running, not a good result. */
    readonly severity: 'ok' | 'warn' | 'bad' | 'muted';
    readonly chip: string;
    /** The primary button's label. */
    readonly action: string;
    readonly kind: PlateAction;
}

/**
 * What one plate says, from its status alone.
 *
 * The chip carries the whole verdict rather than a per-check summary, because the preflight
 * strip beside it already shows where a failure happened. This says what to do about it.
 */
export class PlateView {
    static of(entry: ServerRosterEntry): PlateViewState {
        switch (entry.status) {
            case 'connect':
                return entry.slow
                    ? { chip: 'Ready · slow', severity: 'warn', action: 'Connect', kind: 'connect' }
                    : { chip: 'Ready', severity: 'ok', action: 'Connect', kind: 'connect' };

            case 'reauth':
                return {
                    chip: 'Sign in needed',
                    severity: 'bad',
                    action: 'Sign in',
                    kind: 'signin',
                };

            /**
             * Both directions are blocked, and only one of them an update can fix. The
             * design's table treats them as one chip; pointing "Update needed" at somebody
             * whose client is the newer of the two sends them to look for an update that
             * does not exist and would not help.
             */
            case 'version_mismatch':
                return entry.clientTooOld
                    ? { chip: 'Update needed', severity: 'warn', action: 'Update', kind: 'blocked' }
                    : {
                          chip: 'Server is older',
                          severity: 'warn',
                          action: 'Blocked',
                          kind: 'blocked',
                      };

            // Connecting would fail, so rechecking is the only thing worth offering.
            case 'udp_blocked':
                return {
                    chip: 'Voice blocked',
                    severity: 'bad',
                    action: 'Recheck',
                    kind: 'recheck',
                };

            case 'unreachable':
                return {
                    chip: 'Not answering',
                    severity: 'bad',
                    action: 'Recheck',
                    kind: 'recheck',
                };

            case 'checking':
                return {
                    chip: 'Checking…',
                    severity: 'muted',
                    action: 'Connect',
                    kind: 'blocked',
                };
        }
    }

    /** Whether choosing this plate leads to the dashboard rather than anywhere else. */
    static isJoinable(entry: ServerRosterEntry): boolean {
        return entry.status === 'connect';
    }

    /**
     * The bar under the list: how many servers are in each state, in one line.
     *
     * Counted rather than listed, because the plates above already name them. This is for
     * the glance that tells you whether the screen is worth reading closely.
     */
    static tally(entries: readonly ServerRosterEntry[]): readonly TallyItem[] {
        const labels = entries.map((entry) => PlateView.tallyLabel(entry));
        const seen = labels.filter((label, index) => labels.indexOf(label) === index);
        return seen.map((label) => {
            const first = entries[labels.indexOf(label)];
            return {
                label,
                count: labels.filter((other) => other === label).length,
                severity: PlateView.tallySeverity(first),
            };
        });
    }

    private static tallyLabel(entry: ServerRosterEntry): string {
        switch (entry.status) {
            case 'connect':
                return entry.slow ? 'slow' : 'ready';
            case 'reauth':
                return 'need sign-in';
            case 'version_mismatch':
                return 'outdated';
            case 'udp_blocked':
                return 'voice blocked';
            case 'unreachable':
                return 'not answering';
            case 'checking':
                return 'checking';
        }
    }

    private static tallySeverity(entry: ServerRosterEntry): TallyItem['severity'] {
        switch (entry.status) {
            case 'connect':
                return entry.slow ? 'warn' : 'ok';
            case 'version_mismatch':
                return 'warn';
            case 'reauth':
            case 'udp_blocked':
            case 'unreachable':
                return 'bad';
            case 'checking':
                return 'busy';
        }
    }
}
