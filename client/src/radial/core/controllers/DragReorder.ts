/**
 * Drag rows to change their order.
 *
 * Reports the new order rather than mutating a caller's array, so the source of truth
 * stays wherever it already was.
 *
 * Manual order and column sorting are mutually exclusive by design: a list cannot
 * honestly be in two orders at once, so choosing one clears the other. Enforcing that
 * is the caller's job, because only the caller knows what "the other" is.
 */
export class DragReorder {
  readonly list: HTMLElement;

  #source: HTMLElement | null = null;
  #onReorder?: (keys: string[]) => void;

  constructor(list: HTMLElement, onReorder?: (keys: string[]) => void) {
    this.list = list;
    this.#onReorder = onReorder;
    for (const row of this.#rows()) this.#wire(row);
  }

  /** Current order, from each row's `data-rad-key`. */
  order(): string[] {
    return this.#rows().map((row) => row.dataset.radKey ?? "");
  }

  /** Re-wire after the list has been rebuilt. */
  refresh(): void {
    for (const row of this.#rows()) this.#wire(row);
  }

  destroy(): void {
    this.#source = null;
  }

  #rows(): HTMLElement[] {
    return [...this.list.querySelectorAll<HTMLElement>(".rad-drag-row")];
  }

  #wire(row: HTMLElement): void {
    if (row.dataset.radDragWired === "true") return;
    row.dataset.radDragWired = "true";
    row.draggable = true;

    row.addEventListener("dragstart", (e) => {
      this.#source = row;
      row.classList.add("is-dragging");
      if (e.dataTransfer) {
        e.dataTransfer.effectAllowed = "move";
        // Firefox refuses to start a drag without payload, even unused payload.
        try {
          e.dataTransfer.setData("text/plain", row.dataset.radKey ?? "");
        } catch {
          /* ignored */
        }
      }
    });

    row.addEventListener("dragend", () => {
      row.classList.remove("is-dragging");
      this.#clearTargets();
      this.#source = null;
    });

    row.addEventListener("dragover", (e) => {
      e.preventDefault();
      if (!this.#source || this.#source === row) return;
      this.#clearTargets();
      row.classList.add("is-over");
    });

    row.addEventListener("drop", (e) => {
      e.preventDefault();
      const source = this.#source;
      if (!source || source === row) return;
      const rows = this.#rows();
      // Dropping onto a row below inserts after it, onto one above inserts before:
      // the row lands where the indicator was drawn.
      if (rows.indexOf(source) < rows.indexOf(row)) row.after(source);
      else row.before(source);
      this.#clearTargets();
      this.#onReorder?.(this.order());
    });
  }

  #clearTargets(): void {
    for (const row of this.#rows()) row.classList.remove("is-over");
  }
}
