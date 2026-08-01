import { StepFlow } from "./StepFlow";

/**
 * Swap between screens inside one frame.
 *
 * For the reference pages, where onboarding, the gate, sign-in and the dead end all
 * live in a single frame so the flow between them can be walked. In the app these are
 * SvelteKit routes; the router that replaces this one still has to re-trigger the entry
 * stagger on arrival, which is why that is a method here rather than inline.
 */
export class ScreenRouter {
  readonly frame: HTMLElement;

  #onChange?: (name: string) => void;

  constructor(frame: HTMLElement, onChange?: (name: string) => void) {
    this.frame = frame;
    this.#onChange = onChange;

    for (const el of frame.querySelectorAll<HTMLElement>("[data-rad-goto]")) {
      el.addEventListener("click", () => this.go(el.dataset.radGoto ?? ""));
    }
  }

  get current(): string {
    return this.frame.querySelector<HTMLElement>(".rad-screen.is-on")?.dataset.radScreen ?? "";
  }

  go(name: string): void {
    for (const screen of this.frame.querySelectorAll<HTMLElement>("[data-rad-screen]")) {
      const on = screen.dataset.radScreen === name;
      if (on && !screen.classList.contains("is-on")) {
        screen.classList.add("is-on");
        screen.scrollTop = 0;
        // Skip anything inside a hidden step: it will get its own stagger when that
        // step is shown, and animating it now spends the entrance on nobody.
        for (const el of screen.querySelectorAll<HTMLElement>(".rad-rise")) {
          if (el.closest("[data-rad-step-body][hidden]")) continue;
          el.style.animation = "none";
          void el.offsetWidth;
          el.style.animation = "";
        }
      } else if (!on) {
        screen.classList.remove("is-on");
      }
    }
    this.#onChange?.(name);
  }

  /** Replay the stagger on the current screen. */
  restage(): void {
    const screen = this.frame.querySelector<HTMLElement>(".rad-screen.is-on");
    if (screen) StepFlow.restartStagger(screen);
  }
}
