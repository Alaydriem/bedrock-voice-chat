/**
 * `unreachable` is deliberately not folded into `reauth`. They lead to different places:
 * one sends someone to sign in again, the other to ask whoever runs the server. Offering a
 * sign-in for a server that is not answering is an invitation to fail.
 */
export type ServerHealthStatus =
    | 'connect'
    | 'reauth'
    | 'version_mismatch'
    | 'missing'
    | 'unreachable';
