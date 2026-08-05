/**
 * What a page decided to be, once it knows.
 *
 * `handoff` is not the same as `show`: a deep link that has been routed is already
 * navigating, and the boot overlay has to stay up over it. Dismissing the overlay there
 * flashes a half-built screen on the way past.
 *
 * Returned rather than performed, so the decision is assertable without a browser.
 */
export type ScreenLanding =
    | { kind: 'navigate'; href: string }
    | { kind: 'show' }
    | { kind: 'handoff' };
