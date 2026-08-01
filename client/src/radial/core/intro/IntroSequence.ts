import { AnimationLoop } from "../canvas/AnimationLoop";
import { Surface } from "../canvas/Surface";
import { Visibility } from "../canvas/Visibility";
import { Color } from "../color/Color";
import { Ease } from "../math/Ease";
import { Hash } from "../math/Hash";
import { MarkRenderer } from "../mark/MarkRenderer";
import { RingGeometry } from "../ring/RingGeometry";
import { RingRenderer } from "../ring/RingRenderer";
import { BRAND_LIFT, INTRO_DEFAULTS, type IntroConfig } from "./IntroConfig";
import { IntroEnvelope } from "./IntroEnvelope";
import { INTRO_PHASES, IntroMarks } from "./IntroPhases";

const AT = IntroMarks.AT;

/**
 * The boot sequence, and the loader it becomes.
 *
 * `renderAt(t)` is pure in `t`: the same timestamp always produces the same frame.
 * That is what lets one implementation drive a live rAF loop, a scrub bar, a
 * single-frame PNG export and a WebM recording without any of them being a special
 * case, and it is what makes the sequence testable without a canvas.
 */
export class IntroSequence {
  readonly surface: Surface;

  #cfg: IntroConfig;
  #loop: AnimationLoop;
  #stop: (() => void) | null = null;
  #t0 = 0;
  #paused = false;
  #pausedAt = 0;
  #completed = false;
  #reduce: boolean;
  /** Loader only: 0 to 1 collapse progress. Null on the plain sequence path. */
  #collapse: number | null = null;

  onFrame?: (t: number) => void;
  onComplete?: () => void;

  constructor(canvas: HTMLCanvasElement, config: Partial<IntroConfig> = {}, loop?: AnimationLoop) {
    this.surface = new Surface(canvas);
    this.#cfg = { ...INTRO_DEFAULTS, ...config };
    this.#loop = loop ?? AnimationLoop.shared();
    this.#reduce = Visibility.prefersReducedMotion();
    this.#applySize();
  }

  get config(): Readonly<IntroConfig> {
    return this.#cfg;
  }

  /** One full cycle in wall-clock seconds, including the hold. */
  get duration(): number {
    return (AT.total + this.#cfg.holdSeconds) / this.#cfg.speed;
  }

  /** Length of the sequence itself, in sequence seconds, excluding the hold. */
  get sequenceLength(): number {
    return AT.total;
  }

  get isPaused(): boolean {
    return this.#paused;
  }

  get currentTime(): number {
    if (this.#paused) return this.#pausedAt;
    return ((IntroSequence.now() - this.#t0) / 1000) * this.#cfg.speed;
  }

  static now(): number {
    return typeof performance === "undefined" ? Date.now() : performance.now();
  }

  update(patch: Partial<IntroConfig>): void {
    const resizing = patch.width !== undefined || patch.height !== undefined;
    // Changing speed mid-flight must not jump the playhead.
    const held = patch.speed !== undefined && !this.#paused ? this.currentTime : null;
    this.#cfg = { ...this.#cfg, ...patch };
    if (held !== null) this.#t0 = IntroSequence.now() - (held * 1000) / this.#cfg.speed;
    if (resizing) this.#applySize();
    if (this.#paused) this.renderAt(this.#pausedAt);
  }

  /** Play, optionally starting partway through. */
  start(fromSeconds = 0): void {
    this.#collapse = null;
    this.stop();
    this.#paused = false;
    this.#completed = fromSeconds >= AT.total;
    this.#t0 = IntroSequence.now() - (fromSeconds * 1000) / this.#cfg.speed;

    this.#stop = this.#loop.add(() => {
      const elapsed = (IntroSequence.now() - this.#t0) / 1000;
      let t = elapsed * this.#cfg.speed;
      if (this.#cfg.loop && t > AT.total + this.#cfg.holdSeconds) {
        this.#t0 = IntroSequence.now();
        this.#completed = false;
        t = 0;
      }
      this.renderAt(t);
      this.onFrame?.(t);
      if (!this.#completed && t >= AT.total) {
        this.#completed = true;
        this.onComplete?.();
      }
    });
  }

  stop(): void {
    this.#stop?.();
    this.#stop = null;
  }

  restart(): void {
    this.start(0);
  }

  /** Freeze on one timestamp, in sequence seconds. */
  seek(t: number): void {
    this.stop();
    this.#paused = true;
    this.#pausedAt = t;
    this.renderAt(t);
    this.onFrame?.(t);
  }

  /**
   * Run as an indefinite loading indicator: the mark dances until `finishCollapse`.
   * @param withIntro play the full boot sequence first. Otherwise it opens on the
   *   steady dance with the ring already landed, which is what a page transition
   *   wants — the sequence is an arrival, not a wait.
   */
  startLoading(withIntro = false): void {
    this.update({ endState: "dance", loop: false });
    this.start(withIntro ? 0 : AT.total);
  }

  /**
   * Loading finished: collapse the mark to its flat colour row. The ring keeps
   * turning. Resolves when the collapse lands.
   */
  finishCollapse(seconds: number = INTRO_PHASES.settle): Promise<void> {
    return new Promise((resolve) => {
      const t0 = IntroSequence.now();
      this.#collapse = 0;
      const step = () => {
        const p = Ease.clamp01((IntroSequence.now() - t0) / (seconds * 1000));
        this.#collapse = p;
        if (this.#paused) this.renderAt(this.#pausedAt);
        if (p < 1) requestAnimationFrame(step);
        else resolve();
      };
      requestAnimationFrame(step);
    });
  }

  /** The amplitude and beat maths, as a value with no canvas attached. */
  get envelope(): IntroEnvelope {
    return new IntroEnvelope(this.#cfg.endState, this.#cfg.idleGain);
  }

  /** Beat envelope through the pulse phase: three decaying hits. */
  beat(t: number): number {
    return this.envelope.beat(t);
  }

  /** Mark amplitude across the timeline. */
  gain(t: number): number {
    return this.envelope.gain(t);
  }

  /** Amplitude for a frame, including a loader collapse in progress. */
  liveGain(t: number): number {
    if (this.#collapse === null) return this.gain(t);
    return this.envelope.collapseGain(this.#collapse);
  }

  renderAt(t: number): void {
    const cfg = this.#cfg;
    const reduce = this.#reduce;
    const w = cfg.width;
    const h = cfg.height;
    // The renderers take milliseconds; the sequence is expressed in seconds.
    const ms = t * 1000;

    const x = this.surface.begin();
    if (cfg.background) {
      x.fillStyle = cfg.background;
      x.fillRect(0, 0, w, h);
    }

    const g = RingGeometry.fit(w, h, cfg.scale);
    const beat = this.beat(t);
    const gain = this.liveGain(t);

    if (t > AT.wave) {
      const p = Ease.phase(t, AT.wave, INTRO_PHASES.implode);
      // Each bar leaves at its own moment, so the collapse reads as many objects
      // arriving rather than one hoop shrinking.
      const travel = (bar: number) => Ease.clamp01((p - Hash.scatter(bar) * 0.22) / 0.78);
      RingRenderer.draw(x, {
        geometry: g,
        t: ms,
        base: Color.mix(cfg.ringIdleColor, cfg.ringColor, Ease.clamp01(gain)),
        hum: 0.07,
        boost: beat * 0.1,
        rot: t * cfg.ringSpin,
        hairline: false,
        offsetFor: (b) => Ease.lerp(g.span * 1.05, 0, Ease.outBack(Ease.outCubic(travel(b)))),
        alphaFor: (b) => Ease.clamp01(travel(b) * 2.4),
        widthFor: (b) => Ease.lerp(2.6, 1, Ease.outCubic(travel(b))),
        reduce,
      });
    }

    const circleIn = Ease.phase(t, AT.implode - INTRO_PHASES.implode * 0.45, INTRO_PHASES.implode * 0.45);
    if (circleIn > 0) {
      x.save();
      x.lineWidth = Math.max(1, g.span * 0.0016);
      x.strokeStyle = "rgba(148,131,182,.42)";
      x.globalAlpha = Ease.inOutCubic(circleIn);
      x.beginPath();
      x.arc(g.cx, g.cy, g.inner * (1 + beat * 0.02), 0, Math.PI * 2);
      x.stroke();
      x.restore();

      // A single flash at the moment the bars land.
      const flash = Ease.phase(t, AT.implode, 0.45);
      if (flash > 0 && flash < 1) {
        x.save();
        x.lineWidth = Math.max(1, g.span * 0.005) * (1 - flash);
        x.strokeStyle = Color.rgba(BRAND_LIFT, (1 - flash) * 0.5);
        x.beginPath();
        x.arc(g.cx, g.cy, g.inner * Ease.lerp(0.94, 1.14, Ease.outCubic(flash)), 0, Math.PI * 2);
        x.stroke();
        x.restore();
      }
    }

    const { cell, gap, width: mw, height: mh } = g.markCell(cfg.logoScale);
    const tintMix = Ease.clamp01(Ease.phase(t, AT.scan, INTRO_PHASES.charge));
    if (cfg.glow && tintMix > 0) {
      x.shadowColor = Color.rgba(BRAND_LIFT, 0.5 * tintMix);
      x.shadowBlur = g.span * 0.03 * tintMix;
    }
    MarkRenderer.draw(x, {
      ox: g.cx - mw / 2,
      oy: g.cy - mh / 2,
      cell,
      gap,
      t: ms,
      gain,
      reveal: Ease.clamp01(t / (INTRO_PHASES.scan * 0.92)),
      tintMix,
      idle: cfg.idleTint,
      tint: cfg.markTint,
      mortar: cfg.mortar,
      mortarColor: cfg.mortarColor,
      reduce,
    });
    x.shadowBlur = 0;
  }

  #applySize(): void {
    this.surface.resize(this.#cfg.width, this.#cfg.height);
  }
}
