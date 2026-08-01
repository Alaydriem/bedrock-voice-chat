import type { Menu, MenuItem } from "./Menu";

/**
 * A select, backed by the shared dropdown.
 *
 * Options live on the element as a pipe-separated `data-rad-select`, which keeps a
 * static reference page honest: the markup shown is the markup that runs. In the app,
 * pass options to the constructor instead.
 *
 *   <button class="rad-select" data-rad-select="Auto|48 kHz|44.1 kHz">
 *     <span class="rad-select__value">Auto</span>
 *     <span data-rad-icon="chev"></span>
 *   </button>
 */
export class SelectControl {
  readonly element: HTMLElement;

  #menu: Menu;
  #options: string[];
  #onPick: ((value: string) => void) | null;

  constructor(
    element: HTMLElement,
    menu: Menu,
    options?: string[],
    onPick?: (value: string) => void,
  ) {
    this.element = element;
    this.#menu = menu;
    this.#options = options ?? (element.dataset.radSelect ?? "").split("|").filter(Boolean);
    this.#onPick = onPick ?? null;

    element.setAttribute("aria-haspopup", "listbox");
    element.setAttribute("aria-expanded", "false");
    element.addEventListener("click", () => this.open());
  }

  get value(): string {
    return this.#valueEl()?.textContent?.trim() ?? "";
  }

  set value(next: string) {
    const el = this.#valueEl();
    if (el) el.textContent = next;
  }

  open(): void {
    const current = this.value;
    const entries: MenuItem[] = this.#options.map((label) => ({ label, on: label === current }));
    this.#menu.open(this.element, entries, (item) => {
      this.value = item.label;
      this.#onPick?.(item.label);
    });
  }

  /** Wire every `[data-rad-select]` under a root. */
  static bindAll(root: ParentNode, menu: Menu, onPick?: (value: string, el: HTMLElement) => void): SelectControl[] {
    return [...root.querySelectorAll<HTMLElement>("[data-rad-select]")].map(
      (el) => new SelectControl(el, menu, undefined, (value) => onPick?.(value, el)),
    );
  }

  #valueEl(): HTMLElement | null {
    return this.element.querySelector<HTMLElement>(".rad-select__value");
  }
}
