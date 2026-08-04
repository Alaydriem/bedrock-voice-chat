/**
 * What the server page decided to be.
 *
 * `handoff` is not the same as `show`: a deep link that has been routed is already
 * navigating, and the boot overlay has to stay up over it. Dismissing the overlay there
 * flashes an empty list on the way past.
 */
export type ServerLanding =
    | { kind: 'navigate'; href: string }
    | { kind: 'show' }
    | { kind: 'handoff' };
