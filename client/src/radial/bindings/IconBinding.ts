import { type IconName, Icons } from "../core/icons/Icons";
import type { Binding } from "./Binding";

/**
 * An icon, in place.
 *
 *   <span data-rad-icon="mic"></span>
 *
 * Swapping the attribute swaps the glyph, which is how a mic becomes a struck-through
 * mic on mute without the button knowing any markup. Re-rendering is skipped when the
 * name has not changed, so calling `sync` on a whole subtree every frame is cheap —
 * though there is rarely a reason to.
 */
export class IconBinding implements Binding {
  /** What is currently painted into each element, so a re-render is skippable
   *  even when the caller does not hold onto the binding. */
  static #rendered = new WeakMap<HTMLElement, string>();

  readonly element: HTMLElement;

  constructor(element: HTMLElement, name?: string) {
    this.element = element;
    if (name) element.dataset.radIcon = name;
    this.render();
  }

  get name(): string {
    return this.element.dataset.radIcon ?? "";
  }

  set name(value: string) {
    this.element.dataset.radIcon = value;
    this.render();
  }

  render(): void {
    const want = this.name;
    if (want === IconBinding.#rendered.get(this.element)) return;
    this.element.innerHTML = Icons.has(want) ? Icons.svg(want as IconName) : "";
    IconBinding.#rendered.set(this.element, want);
  }

  destroy(): void {
    // Nothing to release.
  }

  /** Re-render every icon under a root whose name has changed. */
  static sync(root: ParentNode): void {
    for (const el of root.querySelectorAll<HTMLElement>("[data-rad-icon]")) {
      new IconBinding(el).render();
    }
  }
}
