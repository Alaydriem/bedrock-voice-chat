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
 *
 * A sheet can be dragged down to dismiss from its handle or from any part of it that is
 * scrolled to the top. `rad-sheet--full` opens one to the top of the frame instead of
 * parking it at the bottom.
 */
import { CoverDrag } from "./CoverDrag";

export class Sheet {
  readonly frame: HTMLElement;

  #open: HTMLElement | null = null;
  #returnTo: HTMLElement | null = null;
  #onKey: (e: KeyboardEvent) => void;
  #dragging: HTMLElement | null = null;
  #startY = 0;
  #offset = 0;

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

    for (const el of frame.querySelectorAll<HTMLElement>("[data-rad-sheet]")) {
      this.#drag(el);
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
    for (const sheet of this.#all()) {
      sheet.classList.remove("is-open", "is-dragging");
      sheet.style.transform = "";
    }
    this.#scrim(false);
    this.#open = null;
    this.#returnTo?.focus();
    this.#returnTo = null;
  }

  destroy(): void {
    document.removeEventListener("keydown", this.#onKey);
  }

  /**
   * Drag the handle down to dismiss.
   *
   * The handle was a bar that looked draggable and was not, which is worse than no handle:
   * the one gesture a bottom sheet advertises did nothing, and the way out was the scrim.
   * Bound to the sheet rather than to the bar so a drag anywhere in a sheet scrolled to its
   * top closes it, which is what the same gesture does on the settings cover.
   */
  #drag(sheet: HTMLElement): void {
    sheet.addEventListener("pointerdown", (e) => {
      if (sheet !== this.#open) return;
      const target = e.target as HTMLElement;
      // A press on a row is a press on that row, and a slider owns its own drag.
      if (target.closest("button, a, input, select, .rad-range")) return;
      if (!CoverDrag.canStart(sheet.scrollTop)) return;
      this.#dragging = sheet;
      this.#startY = e.clientY;
      this.#offset = 0;
      try {
        sheet.setPointerCapture(e.pointerId);
      } catch {
        // Synthetic events have no live pointer.
      }
    });

    sheet.addEventListener("pointermove", (e) => {
      if (this.#dragging !== sheet) return;
      const dy = e.clientY - this.#startY;
      if (!CoverDrag.isDrag(dy) && this.#offset === 0) return;
      this.#offset = CoverDrag.offset(dy);
      sheet.classList.add("is-dragging");
      sheet.style.transform = `translateY(${this.#offset}px)`;
    });

    const end = (e: PointerEvent) => {
      if (this.#dragging !== sheet) return;
      const travelled = this.#offset;
      this.#dragging = null;
      this.#offset = 0;
      sheet.classList.remove("is-dragging");
      sheet.style.transform = "";
      try {
        sheet.releasePointerCapture(e.pointerId);
      } catch {
        // Never captured, so nothing to release.
      }
      if (CoverDrag.dismisses(travelled)) this.close();
    };
    sheet.addEventListener("pointerup", end);
    sheet.addEventListener("pointercancel", end);

    // The browser reclaiming the pointer mid-drag — the webview backgrounded, the gesture
    // handed to the system. No pointerup is coming, so the drag ends here: spring back
    // rather than close, because the user never finished asking to leave. After a normal
    // release `end` has already run and the guard makes this a no-op.
    //
    // Only the sheet's own loss counts. On touch, pointerdown implicitly captures the
    // pointer to the element under the finger, and taking it for the sheet makes that
    // child announce the transfer with a lostpointercapture that bubbles up here — the
    // start of every touch drag, not the end of one.
    sheet.addEventListener("lostpointercapture", (e) => {
      if (e.target !== sheet) return;
      if (this.#dragging !== sheet) return;
      this.#dragging = null;
      this.#offset = 0;
      sheet.classList.remove("is-dragging");
      sheet.style.transform = "";
    });
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
