/**
 * The four checks, in the order they run.
 *
 * The first three are what connecting already does, named and timed rather than collapsed
 * into one badge. The fourth is new: every check above it runs over TCP 443, so a network
 * that permits HTTPS and drops UDP passes all three and then cannot carry a single audio
 * frame — which surfaces later as a bare QUIC01.
 *
 * That check is named for voice rather than for QUIC because there is more than one way to
 * carry it. A blocked UDP path is a verdict about the transport, not about the product: the
 * same measurement decides whether voice arrives over QUIC, over the TCP fallback, or not at
 * all, and a row named "QUIC path" could only ever report the first of the three.
 */
export const PREFLIGHT_STEPS = ['Credentials', 'Handshake', 'Protocol', 'Voice path'] as const;

export type PreflightStepName = (typeof PREFLIGHT_STEPS)[number];
