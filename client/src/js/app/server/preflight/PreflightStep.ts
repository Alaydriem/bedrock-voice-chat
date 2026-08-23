import type { PreflightStepName } from './PreflightStepName';
import type { PreflightStepState } from './PreflightStepState';

/** One check, its result, and how long it took. */
export interface PreflightStep {
    readonly name: PreflightStepName;
    readonly state: PreflightStepState;
    /**
     * What the check found, in the terms of the thing it checked. A check's duration says
     * which one was slow; this says what it learned.
     */
    readonly note: string;
    readonly ms: number;
}
