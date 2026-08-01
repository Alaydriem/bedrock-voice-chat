import { Toast } from "./Toast";

export interface FormControlHooks {
  onToggle?: (name: string, on: boolean, el: HTMLElement) => void;
  onSegment?: (group: string, value: string, el: HTMLElement) => void;
  onRadio?: (group: string, value: string, el: HTMLElement) => void;
  onCheckbox?: (name: string, on: boolean, el: HTMLElement) => void;
  onStep?: (name: string, value: number, el: HTMLElement) => void;
  /** Return false to suppress the confirmation toast. */
  onCopy?: (value: string, el: HTMLElement) => boolean | void;
}

/**
 * The small controls, wired by delegation from one listener on the frame.
 *
 * Delegation rather than a listener per control, because every one of these appears in
 * lists that are rebuilt — a table's rows, a rebuilt roster — and per-element listeners
 * would either leak or need re-binding after every render.
 *
 * State lives in the DOM, on the ARIA attribute that already has to be correct:
 * `aria-checked` and `aria-pressed` are the source of truth rather than a mirror of it,
 * so a control cannot look one way and report another.
 */
export class FormControls {
  readonly root: HTMLElement;

  #hooks: FormControlHooks;
  #onClick: (e: Event) => void;

  constructor(root: HTMLElement, hooks: FormControlHooks = {}) {
    this.root = root;
    this.#hooks = hooks;
    this.#onClick = (e) => this.#handle(e);
    root.addEventListener("click", this.#onClick);
  }

  destroy(): void {
    this.root.removeEventListener("click", this.#onClick);
  }

  #handle(e: Event): void {
    const target = e.target as HTMLElement;

    const toggle = target.closest<HTMLElement>("[data-rad-toggle]");
    if (toggle && !(toggle as HTMLButtonElement).disabled) {
      const on = toggle.getAttribute("aria-checked") !== "true";
      toggle.setAttribute("aria-checked", String(on));
      this.#hooks.onToggle?.(toggle.dataset.radToggle ?? "", on, toggle);
      return;
    }

    const segment = target.closest<HTMLElement>("[data-rad-segment]");
    if (segment) {
      const group = segment.closest<HTMLElement>(".rad-segmented");
      const value = segment.dataset.radSegment ?? "";
      if (group) {
        for (const b of group.querySelectorAll<HTMLElement>("[data-rad-segment]")) {
          b.setAttribute("aria-pressed", String(b === segment));
        }
        this.#hooks.onSegment?.(group.dataset.radSegmentGroup ?? "", value, segment);
      }
      return;
    }

    const radio = target.closest<HTMLElement>("[data-rad-radio]");
    if (radio) {
      const group = radio.dataset.radRadio ?? "";
      for (const r of this.root.querySelectorAll<HTMLElement>(`[data-rad-radio="${group}"]`)) {
        r.setAttribute("aria-checked", String(r === radio));
      }
      this.#hooks.onRadio?.(group, radio.dataset.radValue ?? "", radio);
      return;
    }

    const checkbox = target.closest<HTMLElement>(".rad-checkbox");
    if (checkbox) {
      const on = checkbox.getAttribute("aria-checked") !== "true";
      checkbox.setAttribute("aria-checked", String(on));
      this.#hooks.onCheckbox?.(checkbox.dataset.radCheckbox ?? "", on, checkbox);
      return;
    }

    const step = target.closest<HTMLElement>("[data-rad-step]");
    if (step) {
      this.#step(step);
      return;
    }

    const copy = target.closest<HTMLElement>("[data-rad-copy]");
    if (copy) {
      const value = copy.dataset.radCopy ?? "";
      const handled = this.#hooks.onCopy?.(value, copy);
      void navigator.clipboard?.writeText(value).catch(() => {});
      if (handled !== false) Toast.show(`Copied — ${value}`);
      return;
    }

    const disclosure = target.closest<HTMLElement>("[data-rad-disclosure-head]");
    if (disclosure) {
      const host = disclosure.closest<HTMLElement>(".rad-disclosure");
      const open = host?.classList.toggle("is-open") ?? false;
      disclosure.setAttribute("aria-expanded", String(open));
    }
  }

  /** Bounds come off the stepper element, so two steppers never share a clamp. */
  #step(button: HTMLElement): void {
    const stepper = button.closest<HTMLElement>(".rad-stepper");
    const readout = stepper?.querySelector<HTMLElement>("[data-rad-step-value]");
    if (!stepper || !readout) return;
    const min = Number(stepper.dataset.radMin ?? "0");
    const max = Number(stepper.dataset.radMax ?? "100");
    const size = Number(stepper.dataset.radStep ?? "1");
    const direction = Number(button.dataset.radStep ?? "0");
    const next = Math.max(min, Math.min(max, Number(readout.textContent) + direction * size));
    readout.textContent = String(next);
    for (const b of stepper.querySelectorAll<HTMLButtonElement>("[data-rad-step]")) {
      const dir = Number(b.dataset.radStep);
      b.disabled = (dir < 0 && next <= min) || (dir > 0 && next >= max);
    }
    this.#hooks.onStep?.(stepper.dataset.radStepper ?? "", next, stepper);
  }
}
