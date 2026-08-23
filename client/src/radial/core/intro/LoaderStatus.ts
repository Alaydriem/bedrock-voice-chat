export interface LoaderStatusFrame {
  visible: boolean;
  glyph: string;
  phrase: string;
}

export interface LoaderStatusOptions {
  /** Cycled beneath the mark. Empty means the loader says nothing. */
  phrases: readonly string[];
  /** How long a wait has to run before it is worth explaining. */
  slowAfterSeconds?: number;
  phraseSeconds?: number;
  frameSeconds?: number;
}

/**
 * What a loader says when it has been going a while.
 *
 * A dancing mark tells you the app is alive. It cannot tell you which of five
 * things it is waiting on, and after a few seconds that is the question. This is
 * the answer: a braille spinner and a line of text, both derived from elapsed
 * time so one instance drives a live loop, a test, or a scrubber.
 *
 * Pure in `elapsed`: no timers, no DOM, no canvas.
 */
export class LoaderStatus {
  /** The indicator CLI tools render when they are working. */
  static readonly BRAILLE_FRAMES: readonly string[] = [
    "⠋",
    "⠙",
    "⠹",
    "⠸",
    "⠼",
    "⠴",
    "⠦",
    "⠧",
    "⠇",
    "⠏",
  ];

  readonly phrases: readonly string[];
  readonly slowAfterSeconds: number;
  readonly phraseSeconds: number;
  readonly frameSeconds: number;

  constructor(options: LoaderStatusOptions) {
    this.phrases = options.phrases;
    this.slowAfterSeconds = options.slowAfterSeconds ?? 4;
    this.phraseSeconds = options.phraseSeconds ?? 1.6;
    this.frameSeconds = options.frameSeconds ?? 0.12;
  }

  at(elapsedSeconds: number): LoaderStatusFrame {
    if (this.phrases.length === 0 || elapsedSeconds < this.slowAfterSeconds) {
      return { visible: false, glyph: "", phrase: "" };
    }

    const since = elapsedSeconds - this.slowAfterSeconds;
    const glyphs = LoaderStatus.BRAILLE_FRAMES;

    return {
      visible: true,
      glyph: glyphs[Math.floor(since / this.frameSeconds) % glyphs.length],
      phrase: this.phrases[Math.floor(since / this.phraseSeconds) % this.phrases.length],
    };
  }
}
