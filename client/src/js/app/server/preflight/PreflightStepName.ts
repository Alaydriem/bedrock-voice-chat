/**
 * The four checks, in the order they run.
 *
 * The first three are what connecting already does, named and timed rather than collapsed
 * into one badge. The fourth is new: every check above it runs over TCP 443, so a network
 * that permits HTTPS and drops UDP passes all three and then cannot carry a single audio
 * frame — which surfaces later as a bare QUIC01.
 */
export const PREFLIGHT_STEPS = ['Credentials', 'Handshake', 'Protocol', 'QUIC path'] as const;

export type PreflightStepName = (typeof PREFLIGHT_STEPS)[number];
