import type { IntroConfig } from "./IntroConfig";
import { IntroSequence } from "./IntroSequence";

export interface LoaderOptions extends Partial<IntroConfig> {
  /** Play the full boot sequence before settling into the dance. */
  withIntro?: boolean;
}

/**
 * The mark as a loading indicator.
 *
 *   const loader = Loader.mount(canvas);
 *   await fetchServers();
 *   await loader.finish();
 *
 * `finish()` collapses the mark to its flat colour row and resolves when it lands,
 * so the caller can navigate on a completed animation rather than guessing at a
 * timeout. The ring keeps turning throughout: the system is still listening.
 *
 * Background defaults to null so the loader sits on whatever is behind it.
 */
export class Loader {
  readonly sequence: IntroSequence;

  private constructor(sequence: IntroSequence) {
    this.sequence = sequence;
  }

  static mount(canvas: HTMLCanvasElement, options: LoaderOptions = {}): Loader {
    const { withIntro = false, ...rest } = options;
    const sequence = new IntroSequence(canvas, {
      background: null,
      ...rest,
      endState: "dance",
      loop: false,
    });
    sequence.startLoading(withIntro);
    return new Loader(sequence);
  }

  finish(seconds?: number): Promise<void> {
    return this.sequence.finishCollapse(seconds);
  }

  stop(): void {
    this.sequence.stop();
  }
}
