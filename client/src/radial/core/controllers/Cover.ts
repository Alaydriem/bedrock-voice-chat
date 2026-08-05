/**
 * One screen presented over another.
 *
 * Settings over the dashboard: a second screen reached from the first, which is why
 * the first stays mounted and visible at the top edge rather than being replaced. The
 * dashboard behind is doing real work — a link, a roster, a position feed — and
 * tearing it down to show a settings pane means rebuilding all of it on the way back.
 *
 *   <div class="rad-under" data-rad-under> … the dashboard … </div>
 *   <div class="rad-scrim rad-scrim--cover" data-rad-cover-scrim></div>
 *   <div class="rad-cover" data-rad-cover="settings"> … </div>
 *   <button data-rad-cover-open="settings">…</button>
 *
 * Dismissal is a request, not an instruction. A cover with levels inside it — the
 * phone's section list and its detail — has to spend the first Escape or back gesture
 * climbing to its top level, because that is what the same gesture does everywhere
 * else. `onDismiss` returns true to absorb one.
 */
export interface CoverOptions {
  /** Return true to keep the cover open and handle the gesture internally. */
  onDismiss?: () => boolean;
  onOpen?: (name: string) => void;
  onClose?: (name: string) => void;
}

export class Cover {
  readonly frame: HTMLElement;

  #open: HTMLElement | null = null;
  #returnTo: HTMLElement | null = null;
  #options: CoverOptions;
  #onKey: (e: KeyboardEvent) => void;

  constructor(frame: HTMLElement, options: CoverOptions = {}) {
    this.frame = frame;
    this.#options = options;

    for (const el of frame.querySelectorAll<HTMLElement>("[data-rad-cover-open]")) {
      el.addEventListener("click", () => this.open(el.dataset.radCoverOpen ?? "", el));
    }
    // An explicit close means close. Only the ambient gestures — Escape, the scrim, the
    // platform's back — are a request to go back one, which inside a cover with levels
    // is not the same thing as leaving it.
    for (const el of frame.querySelectorAll<HTMLElement>("[data-rad-cover-close]")) {
      el.addEventListener("click", () => this.close());
    }
    for (const el of frame.querySelectorAll<HTMLElement>("[data-rad-cover-scrim]")) {
      el.addEventListener("click", () => this.dismiss());
    }

    this.#onKey = (e) => {
      if (e.key !== "Escape" || !this.#open) return;
      // A menu or a modal on top of the cover owns Escape first: the cover is the
      // surface they were opened from, so closing it out from under them would
      // dismiss two things with one press.
      if (this.frame.querySelector(".rad-menu.is-open, .rad-modal.is-open")) return;
      e.preventDefault();
      this.dismiss();
    };
    document.addEventListener("keydown", this.#onKey);
  }

  get openName(): string | null {
    return this.#open?.dataset.radCover ?? null;
  }

  get isOpen(): boolean {
    return this.#open !== null;
  }

  open(name: string, returnTo?: HTMLElement): void {
    const target = this.frame.querySelector<HTMLElement>(`[data-rad-cover="${name}"]`);
    if (!target || target === this.#open) return;
    this.#returnTo = returnTo ?? (document.activeElement as HTMLElement | null);
    for (const cover of this.#all()) cover.classList.toggle("is-open", cover === target);
    this.#under(true);
    this.#open = target;
    this.#options.onOpen?.(name);
  }

  /** The back gesture: the host gets first refusal, then the cover closes. */
  dismiss(): void {
    if (!this.#open) return;
    if (this.#options.onDismiss?.()) return;
    this.close();
  }

  close(): void {
    const name = this.openName;
    for (const cover of this.#all()) cover.classList.remove("is-open");
    this.#under(false);
    this.#open = null;
    this.#returnTo?.focus();
    this.#returnTo = null;
    if (name) this.#options.onClose?.(name);
  }

  destroy(): void {
    document.removeEventListener("keydown", this.#onKey);
  }

  #all(): HTMLElement[] {
    return [...this.frame.querySelectorAll<HTMLElement>("[data-rad-cover]")];
  }

  /**
   * The screen behind, pushed back and taken out of the tab order.
   *
   * Scaling it alone leaves every control under the cover still focusable, so tabbing
   * off the end of the settings pane lands on a dashboard button nobody can see.
   */
  #under(covered: boolean): void {
    for (const el of this.frame.querySelectorAll<HTMLElement>("[data-rad-under]")) {
      el.classList.toggle("is-covered", covered);
      el.inert = covered;
    }
    for (const el of this.frame.querySelectorAll<HTMLElement>("[data-rad-cover-scrim]")) {
      el.classList.toggle("is-on", covered);
    }
  }
}
