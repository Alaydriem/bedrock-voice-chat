/** One state and how many servers are in it, for the bar under the list. */
export interface TallyItem {
    readonly label: string;
    readonly count: number;
    /** `busy` is a check still running, which is not a result. */
    readonly severity: 'ok' | 'warn' | 'bad' | 'busy';
}
