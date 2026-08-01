/**
 * A speech envelope for a recorded lane.
 *
 * Deterministic in (lane, index), so scrolling the timeline reveals a stable
 * waveform rather than noise that changes when you look away. The gate term
 * produces silences, because a track that is loud everywhere does not read as
 * someone talking.
 */
export class TimelineEnvelope {
  static at(lane: number, index: number): number {
    const a = Math.sin(index * 0.21 + lane * 2.1);
    const b = Math.sin(index * 0.071 + lane * 4.7);
    const c = Math.sin(index * 0.53 + lane * 1.3);
    const v = Math.abs(a * 0.5 + b * 0.3 + c * 0.2);
    const gate = Math.sin(index * 0.029 + lane * 2.7);
    return gate > 0.05 ? Math.min(1, v * 1.5 * Math.min(1, gate * 2.4)) : 0;
  }
}
