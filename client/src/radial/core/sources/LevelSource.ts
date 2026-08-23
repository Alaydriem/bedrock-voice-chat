export type LevelListener = (level: number) => void;
/** Call to stop listening. */
export type Unsubscribe = () => void;

/**
 * Somewhere a level comes from, 0 to 1.
 *
 * The only contract between the UI and the audio layer. Every meter, and every
 * voice placed on the ring, consumes this and nothing else — so a meter driven by
 * a Tauri event, by a WebSocket, by a test, or by the synthetic oscillator in the
 * reference pages is the same component with a different constructor argument.
 *
 * Implementations push; they are never polled.
 */
export interface LevelSource {
  /** Register a listener. Returns the function that unregisters it. */
  subscribe(listener: LevelListener): Unsubscribe;
  /** Most recent level, for a first paint before the next push arrives. */
  readonly level: number;
}

/**
 * A source someone else drives. This is the adapter seam: a Tauri event handler or
 * a WebSocket message handler holds one of these and calls `push`.
 *
 *   const mic = new PushLevelSource();
 *   listen<number>("audio://input-level", (e) => mic.push(e.payload));
 *   new LevelMeterBinding(canvas, { source: mic });
 */
export class PushLevelSource implements LevelSource {
  #listeners = new Set<LevelListener>();
  #level = 0;

  get level(): number {
    return this.#level;
  }

  /**
   * How many listeners this source is feeding.
   *
   * A binding keeps whichever source object it was handed at construction, and nothing in the
   * window can otherwise see that the object being pushed to is no longer that one. Counted
   * here because this is the only place that knows.
   */
  get listeners(): number {
    return this.#listeners.size;
  }

  subscribe(listener: LevelListener): Unsubscribe {
    this.#listeners.add(listener);
    listener(this.#level);
    return () => this.#listeners.delete(listener);
  }

  push(level: number): void {
    const clamped = level < 0 ? 0 : level > 1 ? 1 : level;
    this.#level = clamped;
    for (const listener of this.#listeners) listener(clamped);
  }

  /** Drop every listener. Call from a component's teardown. */
  close(): void {
    this.#listeners.clear();
  }
}

/** A source that never moves. For a muted player, or a static specimen. */
export class ConstantLevelSource implements LevelSource {
  readonly level: number;

  constructor(level = 0) {
    this.level = level;
  }

  subscribe(listener: LevelListener): Unsubscribe {
    listener(this.level);
    return () => {};
  }
}
