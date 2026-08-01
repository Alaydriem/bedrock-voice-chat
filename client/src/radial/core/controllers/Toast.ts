/**
 * A confirmation that removes itself.
 *
 * Only ever says what just happened. Nothing that matters can live in something that
 * disappears after 1.7 seconds and cannot be recalled — a failure gets a banner, a
 * decision gets a modal.
 */
export class Toast {
  static readonly DURATION = 1700;

  static #instance: Toast | null = null;

  #element: HTMLElement | null = null;
  #timer: ReturnType<typeof setTimeout> | null = null;
  readonly host: HTMLElement;

  constructor(host: HTMLElement = document.body) {
    this.host = host;
  }

  /** The page-level toast. */
  static shared(): Toast {
    Toast.#instance ??= new Toast();
    return Toast.#instance;
  }

  static show(message: string): void {
    Toast.shared().show(message);
  }

  show(message: string): void {
    if (!this.#element) {
      this.#element = document.createElement("div");
      this.#element.className = "rad-toast";
      this.#element.setAttribute("role", "status");
      // Polite: a confirmation should not interrupt whatever a screen reader is
      // already in the middle of saying.
      this.#element.setAttribute("aria-live", "polite");
      this.host.appendChild(this.#element);
    }
    this.#element.textContent = message;
    requestAnimationFrame(() => this.#element?.classList.add("is-on"));
    if (this.#timer) clearTimeout(this.#timer);
    this.#timer = setTimeout(() => this.#element?.classList.remove("is-on"), Toast.DURATION);
  }

  destroy(): void {
    if (this.#timer) clearTimeout(this.#timer);
    this.#element?.remove();
    this.#element = null;
  }
}
