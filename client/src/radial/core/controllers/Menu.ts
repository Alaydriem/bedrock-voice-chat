import { IconBinding } from "../../bindings/IconBinding";

export interface MenuItem {
  label: string;
  /** Shown right-aligned: a format, a shortcut, a qualifier. */
  hint?: string;
  /** Ticked, for a select's current value. */
  on?: boolean;
  /** Red, and by convention the last item in the list. */
  danger?: boolean;
  disabled?: boolean;
  /** Anything the caller wants back in the pick callback. */
  value?: unknown;
}

/** A divider between groups of items. */
export const MENU_DIVIDER = "-" as const;

/**
 * A labelled heading above a run of items.
 *
 * For a select whose options come from more than one source and where the source
 * changes what picking one means — an audio device under WASAPI and the same box
 * under ASIO are different latencies, not a duplicate entry. A divider alone would
 * separate them without saying which is which.
 */
export interface MenuSection {
  section: string;
}

export type MenuEntry = MenuItem | MenuSection | typeof MENU_DIVIDER;

function isSection(entry: MenuEntry): entry is MenuSection {
  return entry !== MENU_DIVIDER && "section" in entry;
}

/**
 * One dropdown, two jobs.
 *
 * As a select it shows tick marks against the current value. As a table row menu it
 * uses dividers and puts a single destructive item last — always last, and always the
 * only red thing in the list, so "the red one at the bottom" is a reliable shape
 * rather than something to read carefully each time.
 *
 * Positioned against whatever opened it and flipped upward near the bottom edge,
 * inside the frame rather than on the page: the client is a window, not a document.
 */
export class Menu {
  readonly element: HTMLElement;
  readonly frame: HTMLElement;

  #entries: MenuEntry[] = [];
  #onPick: ((item: MenuItem) => void) | null = null;
  #anchor: HTMLElement | null = null;
  #focused = -1;
  #outside: ((e: PointerEvent) => void) | null = null;

  constructor(element: HTMLElement, frame: HTMLElement) {
    this.element = element;
    this.frame = frame;
    this.element.setAttribute("role", "menu");
    this.element.addEventListener("click", (e) => this.#onClick(e));
    document.addEventListener("keydown", (e) => this.#onKey(e));
  }

  get isOpen(): boolean {
    return this.element.classList.contains("is-open");
  }

  open(anchor: HTMLElement, entries: MenuEntry[], onPick: (item: MenuItem) => void): void {
    this.#entries = entries;
    this.#onPick = onPick;
    this.#anchor = anchor;
    this.#focused = -1;
    this.element.innerHTML = entries.map((entry, i) => Menu.#render(entry, i)).join("");
    IconBinding.sync(this.element);
    this.element.classList.add("is-open");
    anchor.setAttribute("aria-expanded", "true");
    this.#position(anchor);

    // Deferred: the click that opened the menu is still propagating, and binding
    // now would close it immediately.
    setTimeout(() => {
      this.#outside = (e: PointerEvent) => {
        if (!this.element.contains(e.target as Node)) this.close();
      };
      document.addEventListener("pointerdown", this.#outside, true);
    }, 0);
  }

  close(): void {
    this.element.classList.remove("is-open");
    this.#anchor?.setAttribute("aria-expanded", "false");
    this.#anchor = null;
    if (this.#outside) {
      document.removeEventListener("pointerdown", this.#outside, true);
      this.#outside = null;
    }
  }

  destroy(): void {
    this.close();
    this.element.innerHTML = "";
  }

  static #render(entry: MenuEntry, index: number): string {
    if (entry === MENU_DIVIDER) return '<div class="rad-menu__divider" role="separator"></div>';
    if (isSection(entry)) {
      return `<div class="rad-menu__section" role="presentation">${Menu.#escape(entry.section)}</div>`;
    }
    const classes = ["rad-menu__item"];
    if (entry.danger) classes.push("rad-menu__item--danger");
    const tick = entry.on
      ? '<span class="rad-menu__tick" data-rad-icon="check"></span>'
      : '<span class="rad-menu__tick rad-menu__tick--empty"></span>';
    const hint = entry.hint ? `<span class="rad-menu__hint">${Menu.#escape(entry.hint)}</span>` : "";
    return (
      `<button class="${classes.join(" ")}" role="menuitem" data-rad-mi="${index}"` +
      `${entry.disabled ? " disabled" : ""} aria-checked="${entry.on === true}">` +
      `${tick}<span>${Menu.#escape(entry.label)}</span>${hint}</button>`
    );
  }

  /** Item labels can be a recording name or a device name — never trusted as markup. */
  static #escape(text: string): string {
    const div = document.createElement("div");
    div.textContent = text;
    return div.innerHTML;
  }

  #items(): HTMLButtonElement[] {
    return [...this.element.querySelectorAll<HTMLButtonElement>("[data-rad-mi]:not([disabled])")];
  }

  #onClick(e: Event): void {
    const button = (e.target as HTMLElement).closest<HTMLElement>("[data-rad-mi]");
    if (!button) return;
    const entry = this.#entries[Number(button.dataset.radMi)];
    if (entry === MENU_DIVIDER || isSection(entry)) return;
    const pick = this.#onPick;
    this.close();
    pick?.(entry);
  }

  #onKey(e: KeyboardEvent): void {
    if (!this.isOpen) return;
    const items = this.#items();
    if (e.key === "Escape") {
      e.preventDefault();
      const anchor = this.#anchor;
      this.close();
      anchor?.focus();
      return;
    }
    if (e.key === "ArrowDown" || e.key === "ArrowUp") {
      e.preventDefault();
      const step = e.key === "ArrowDown" ? 1 : -1;
      this.#focused = (this.#focused + step + items.length) % items.length;
      for (const [i, item] of items.entries()) item.classList.toggle("is-focused", i === this.#focused);
      items[this.#focused]?.focus();
    }
  }

  /**
   * Anchored to the trigger's left edge, below it, clamped inside the frame, and
   * flipped above the trigger when there is no room underneath.
   *
   * Offsets are measured against the menu's own `offsetParent`, because that is what
   * `left` and `top` are actually relative to. Measuring against the frame instead
   * puts the menu wherever the frame happens to sit inside its positioning context —
   * on a centred page that is off by the auto margin, so every menu lands at the same
   * wrong place regardless of what opened it.
   */
  #position(anchor: HTMLElement): void {
    const host = (this.element.offsetParent as HTMLElement | null) ?? this.frame;
    const hostRect = host.getBoundingClientRect();
    const rect = anchor.getBoundingClientRect();
    const width = this.element.offsetWidth;
    const height = this.element.offsetHeight;

    const left = Math.min(rect.left - hostRect.left, hostRect.width - width - 12);
    let top = rect.bottom - hostRect.top + 6;
    if (top + height > hostRect.height - 12) top = rect.top - hostRect.top - height - 6;

    this.element.style.left = `${Math.max(12, left)}px`;
    this.element.style.top = `${Math.max(12, top)}px`;
  }
}
