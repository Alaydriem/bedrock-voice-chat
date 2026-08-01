/**
 * A fixed-length ring buffer of samples, written at one end.
 *
 * The scope draws 72 bars for the last 72 seconds of round-trip time. Ageing from
 * the write head rather than from index zero means exactly one bar changes per
 * tick: the trace decays around the circle instead of the whole ring flickering.
 */
export class ScopeBuffer {
  readonly size: number;
  #samples: number[];
  #head = 0;

  constructor(size: number, fill = 0) {
    this.size = size;
    this.#samples = new Array<number>(size).fill(fill);
  }

  /** Index the next write lands on. */
  get head(): number {
    return this.#head;
  }

  push(value: number): void {
    this.#samples[this.#head] = value;
    this.#head = (this.#head + 1) % this.size;
  }

  at(index: number): number {
    return this.#samples[index] ?? 0;
  }

  /** Ticks since a slot was written: 0 is the newest, size-1 the oldest. */
  age(index: number): number {
    return (this.#head - 1 - index + this.size) % this.size;
  }

  /** Freshness, 1 at the write head decaying to 0 at the oldest sample. */
  life(index: number): number {
    return 1 - this.age(index) / this.size;
  }

  reset(fill = 0): void {
    this.#samples.fill(fill);
    this.#head = 0;
  }
}
