/**
 * Whether a renderer that has nothing new to show still has to draw.
 *
 * A canvas keeps what was last drawn on it, so a meter sitting at rest is already displaying
 * the correct picture and redrawing it sixty times a second produces the same pixels. On a
 * roster where three people out of forty are speaking, that is the overwhelming majority of the
 * work — and it lands on the same thread that lays out the page and drains the events those
 * meters are waiting for, so it is not merely wasted, it is actively in the way.
 *
 * The gate is deliberately conservative: it never suppresses the *first* frame at rest, because
 * that is the one that draws the resting picture, and it repaints on demand whenever something
 * changes that the existing pixels cannot represent.
 */
export class RestGate {
  #painted = false;

  /**
   * Whether this frame must be drawn.
   *
   * `atRest` is the renderer's own judgement that its output would not differ from last time.
   * A gate that has not yet drawn a resting frame always says yes, so coming to rest is
   * rendered once rather than left frozen partway.
   */
  needsPaint(atRest: boolean): boolean {
    if (!atRest) {
      this.#painted = false;
      return true;
    }
    if (this.#painted) return false;
    return true;
  }

  /**
   * Record that a frame reached the canvas.
   *
   * Separate from `needsPaint` because a renderer can decline to draw after asking — an
   * offscreen or zero-sized canvas, most often. Marking it painted there would leave the gate
   * believing a resting picture exists on a canvas that has never been drawn on at all, and it
   * would stay blank for as long as it stayed quiet.
   */
  painted(atRest: boolean): void {
    this.#painted = atRest;
  }

  /**
   * Something changed that the drawn pixels cannot show: a colour, a resize, a new source.
   *
   * Without this a resting meter keeps a stale picture indefinitely, since nothing about the
   * level changed to make it redraw.
   */
  invalidate(): void {
    this.#painted = false;
  }
}
