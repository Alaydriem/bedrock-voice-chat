/**
 * `skipped` is deliberately distinct from `pending`: a check that never ran because an
 * earlier one failed must not leave a reader waiting for a result that is not coming.
 */
export type PreflightStepState = 'pending' | 'running' | 'ok' | 'warn' | 'bad' | 'skipped';
