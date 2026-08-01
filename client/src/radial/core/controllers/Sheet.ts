/**
 * Bottom sheets.
 *
 * The phone stand-in for the rail and the group panel. Rows carry a `--i` index so
 * they enter staggered: the sheet reads as assembling rather than as a block sliding
 * up, which also makes the list scannable a beat sooner.
 *
 *   <div class="rad-scrim" data-rad-sheet-scrim></div>
 *   <div class="rad-sheet" data-rad-sheet="servers"> … </div>
 *   <button data-rad-sheet-open="servers">…</button>
 */
export class Sheet {
  readonly frame: HTMLElement;

  #open: HTMLElement | null = null;
  #returnTo: HTMLElement | null = null;
  #onKey: (e: KeyboardEvent) => void;

  constructor(frame: HTMLElement) {
    this.frame = frame;

    for (const el of frame.querySelectorAll<HTMLElement>("[data-rad-sheet-open]")) {
      el.addEventListener("click", () => this.open(el.dataset.radSheetOpen ?? "", el));
    }
    for (const el of frame.querySelectorAll<HTMLElement>("[data-rad-sheet-close]")) {
      el.addEventListener("click", () => this.close());
    }
    for (const el of frame.querySelectorAll<HTMLElement>("[data-rad-sheet-scrim]")) {
      el.addEventListener("click", () => this.close());
    }

    this.#onKey = (e) => {
      if (e.key === "Escape" && this.#open) {
        e.preventDefault();
        this.close();
      }
    };
    document.addEventListener("keydown", this.#onKey);
  }

  get openName(): string | null {
    return this.#open?.dataset.radSheet ?? null;
  }

  open(name: string, returnTo?: HTMLElement): void {
    const target = this.frame.querySelector<HTMLElement>(`[data-rad-sheet="${name}"]`);
    if (!target) return;
    this.#returnTo = returnTo ?? (document.activeElement as HTMLElement | null);
    Sheet.stagger(target);
    for (const sheet of this.#all()) sheet.classList.toggle("is-open", sheet === target);
    this.#scrim(true);
    this.#open = target;
  }

  close(): void {
    for (const sheet of this.#all()) sheet.classList.remove("is-open");
    this.#scrim(false);
    this.#open = null;
    this.#returnTo?.focus();
    this.#returnTo = null;
  }

  destroy(): void {
    document.removeEventListener("keydown", this.#onKey);
  }

  /**
   * Number the rows for the entry stagger. Called on open rather than once at
   * construction, because a sheet's contents are usually rebuilt between openings.
   */
  static stagger(sheet: HTMLElement): void {
    const rows = sheet.querySelectorAll<HTMLElement>(".rad-sheet-row, .rad-list-row, .rad-group-row");
    rows.forEach((row, i) => row.style.setProperty("--i", String(Math.min(i, 7))));
  }

  #all(): HTMLElement[] {
    return [...this.frame.querySelectorAll<HTMLElement>("[data-rad-sheet]")];
  }

  #scrim(on: boolean): void {
    for (const el of this.frame.querySelectorAll<HTMLElement>("[data-rad-sheet-scrim]")) {
      el.classList.toggle("is-on", on);
    }
  }
}
