export interface StepFlowOptions {
  frame: HTMLElement;
  /** Footer copy, one per step. */
  captions?: readonly string[];
  /** Label for the last step's forward button. */
  finalLabel?: string;
  onStep?: (step: number) => void;
  /** Called when the last step's forward button is pressed. */
  onFinish?: () => void;
}

/**
 * The introduction's step flow.
 *
 * Four steps is the ceiling. Each added step multiplies the skip rate, so a fifth
 * should displace one of these rather than extend the run.
 *
 * Swapping a step re-triggers the entry stagger, which is what makes each step feel
 * like arriving somewhere rather than like content being swapped underneath a frame.
 * Restarting a CSS animation needs the class removed, a layout read to flush it, and
 * the class put back — hence the reflow below, which is load-bearing rather than
 * superstition.
 */
export class StepFlow {
  readonly frame: HTMLElement;
  readonly total: number;

  #options: StepFlowOptions;
  #step = 1;

  constructor(options: StepFlowOptions) {
    this.#options = options;
    this.frame = options.frame;
    this.total = this.frame.querySelectorAll("[data-rad-step-body]").length;

    for (const dot of this.frame.querySelectorAll<HTMLElement>("[data-rad-step-to]")) {
      dot.addEventListener("click", () => this.go(Number(dot.dataset.radStepTo)));
    }
    this.frame.querySelector<HTMLElement>("[data-rad-step-next]")?.addEventListener("click", () => this.next());
    this.frame.querySelector<HTMLElement>("[data-rad-step-back]")?.addEventListener("click", () => this.back());

    this.go(1);
  }

  get step(): number {
    return this.#step;
  }

  next(): void {
    if (this.#step < this.total) this.go(this.#step + 1);
    else this.#options.onFinish?.();
  }

  back(): void {
    if (this.#step > 1) this.go(this.#step - 1);
  }

  go(step: number): void {
    this.#step = Math.max(1, Math.min(this.total, step));

    for (const body of this.frame.querySelectorAll<HTMLElement>("[data-rad-step-body]")) {
      const on = Number(body.dataset.radStepBody) === this.#step;
      body.hidden = !on;
      if (on) StepFlow.restartStagger(body);
    }
    for (const visual of this.frame.querySelectorAll<HTMLElement>("[data-rad-step-visual]")) {
      visual.hidden = Number(visual.dataset.radStepVisual) !== this.#step;
    }
    for (const dot of this.frame.querySelectorAll<HTMLElement>("[data-rad-step-to]")) {
      dot.classList.toggle("is-on", Number(dot.dataset.radStepTo) === this.#step);
    }

    const count = this.frame.querySelector<HTMLElement>("[data-rad-step-count]");
    if (count) {
      count.textContent = `${String(this.#step).padStart(2, "0")} / ${String(this.total).padStart(2, "0")}`;
    }

    const next = this.frame.querySelector<HTMLElement>("[data-rad-step-next]");
    if (next) next.textContent = this.#step === this.total ? (this.#options.finalLabel ?? "Continue") : "Next";

    const back = this.frame.querySelector<HTMLElement>("[data-rad-step-back]");
    if (back) back.hidden = this.#step === 1;

    const caption = this.frame.querySelector<HTMLElement>("[data-rad-step-caption]");
    const captions = this.#options.captions;
    if (caption && captions?.[this.#step - 1]) caption.textContent = captions[this.#step - 1];

    // A step change is a new page as far as the reader is concerned.
    const screen = this.frame.querySelector<HTMLElement>(".rad-screen.is-on");
    if (screen) screen.scrollTop = 0;

    this.#options.onStep?.(this.#step);
  }

  /** Replay the entry animation on every `.rad-rise` inside a subtree. */
  static restartStagger(root: ParentNode): void {
    for (const el of root.querySelectorAll<HTMLElement>(".rad-rise")) {
      el.style.animation = "none";
      void el.offsetWidth;
      el.style.animation = "";
    }
  }
}
