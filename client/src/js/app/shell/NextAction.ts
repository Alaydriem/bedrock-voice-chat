/**
 * Where a decision leads, returned rather than performed.
 *
 * A manager that assigned `window.location` could not be tested without a browser, and the
 * navigation is the part worth asserting.
 */
export type NextAction = { kind: 'navigate'; href: string } | { kind: 'none' };
