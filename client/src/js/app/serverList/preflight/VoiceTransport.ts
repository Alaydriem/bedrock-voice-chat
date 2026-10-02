/**
 * Which transport the voice-path check found, if any.
 *
 * `none` is not an absent measurement — it is the measurement, and the one that makes a
 * server unconnectable. `websocket` is a working path with a cost, so it is neither of the
 * other two.
 */
export type VoiceTransport = 'quic' | 'websocket' | 'none';
