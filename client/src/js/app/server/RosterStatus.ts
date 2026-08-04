/**
 * What a plate has concluded about its server.
 *
 * `checking` is the state before a preflight has finished, which is where every plate
 * starts: a list that waited for the slowest server would show nothing at all while one
 * dead host times out.
 *
 * `udp_blocked` is a hard blocker rather than a warning. Voice is the product, so there is
 * nothing worth connecting to without a UDP path, and the connection would fail anyway.
 *
 * `unreachable` is not in the design's table of five because that table catalogued the
 * states the old card could already produce — and the old card mapped a server that was
 * simply down onto `reauth`, sending people to a sign-in that cannot succeed. Once the
 * handshake is a named check, "it did not answer" and "it refused these credentials" are
 * different failures of that check and cannot share a verdict.
 */
export type RosterStatus =
    | 'checking'
    | 'connect'
    | 'reauth'
    | 'version_mismatch'
    | 'udp_blocked'
    | 'unreachable';
