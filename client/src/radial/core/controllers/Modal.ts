/**
 * Modals, one at a time.
 *
 * A confirm stacked over an export dialog leaves no way to tell which one a Cancel
 * belongs to, so opening a second closes the first. Focus moves in on open, is trapped
 * while open, and returns to whatever opened it on close — without that last part a
 * keyboard user is dropped at the top of the document every time they cancel.
 *
 *   <div class="rad-scrim rad-scrim--modal" data-rad-modal-scrim></div>
 *   <div class="rad-modal" data-rad-modal="confirm-delete"> … </div>
 */
export class Modal {
  static readonly FOCUSABLE =
    'button:not([disabled]), [href], input:not([disabled]), select, textarea, [tabindex]:not([tabindex="-1"])';

  readonly frame: HTMLElement;

  #open: HTMLElement | null = null;
  #returnTo: HTMLElement | null = null;
  #onKey: (e: KeyboardEvent) => void;

  constructor(frame: HTMLElement) {
    this.frame = frame;

    for (const modal of this.#all()) {
      modal.setAttribute("role", "dialog");
      modal.setAttribute("aria-modal", "true");
      const title = modal.querySelector(".rad-modal__title");
      if (title) {
        const id = title.id || `rad-modal-title-${modal.dataset.radModal}`;
        title.id = id;
        modal.setAttribute("aria-labelledby", id);
      }
    }

    for (const el of frame.querySelectorAll<HTMLElement>("[data-rad-modal-close]")) {
      el.addEventListener("click", () => this.close());
    }
    for (const el of frame.querySelectorAll<HTMLElement>("[data-rad-modal-scrim]")) {
      el.addEventListener("click", () => this.close());
    }
    for (const el of frame.querySelectorAll<HTMLElement>("[data-rad-modal-open]")) {
      el.addEventListener("click", () => this.open(el.dataset.radModalOpen ?? "", el));
    }

    this.#onKey = (e) => this.#key(e);
    document.addEventListener("keydown", this.#onKey);
  }

  get openName(): string | null {
    return this.#open?.dataset.radModal ?? null;
  }

  /** @param returnTo element focus goes back to on close. Defaults to the active one. */
  open(name: string, returnTo?: HTMLElement): HTMLElement | null {
    const target = this.frame.querySelector<HTMLElement>(`[data-rad-modal="${name}"]`);
    if (!target) return null;
    this.#returnTo = returnTo ?? (document.activeElement as HTMLElement | null);
    for (const modal of this.#all()) modal.classList.toggle("is-open", modal === target);
    this.#scrim(true);
    this.#open = target;
    target.querySelector<HTMLElement>(Modal.FOCUSABLE)?.focus();
    return target;
  }

  close(): void {
    for (const modal of this.#all()) modal.classList.remove("is-open");
    this.#scrim(false);
    this.#open = null;
    this.#returnTo?.focus();
    this.#returnTo = null;
  }

  destroy(): void {
    document.removeEventListener("keydown", this.#onKey);
  }

  #all(): HTMLElement[] {
    return [...this.frame.querySelectorAll<HTMLElement>("[data-rad-modal]")];
  }

  #scrim(on: boolean): void {
    for (const el of this.frame.querySelectorAll<HTMLElement>("[data-rad-modal-scrim]")) {
      el.classList.toggle("is-on", on);
    }
  }

  #key(e: KeyboardEvent): void {
    if (!this.#open) return;
    if (e.key === "Escape") {
      e.preventDefault();
      this.close();
      return;
    }
    if (e.key !== "Tab") return;
    const focusable = [...this.#open.querySelectorAll<HTMLElement>(Modal.FOCUSABLE)];
    if (focusable.length === 0) return;
    const first = focusable[0];
    const last = focusable[focusable.length - 1];
    if (e.shiftKey && document.activeElement === first) {
      e.preventDefault();
      last.focus();
    } else if (!e.shiftKey && document.activeElement === last) {
      e.preventDefault();
      first.focus();
    }
  }
}
