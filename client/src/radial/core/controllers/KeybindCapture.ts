/**
 * Capture a shortcut by pressing it.
 *
 * Escape cancels and Delete clears, which means neither can be bound — an acceptable
 * trade, because a global hotkey on Escape would fight every dialog in Minecraft.
 *
 * Modifiers are composed in a fixed order so `Ctrl + Shift + M` never renders as
 * `Shift + Ctrl + M`: two spellings of one binding look like two bindings.
 */
export class KeybindCapture {
  static readonly UNSET = "Not set";

  readonly root: HTMLElement;

  #listening: { el: HTMLElement; previous: string } | null = null;
  #onKey: (e: KeyboardEvent) => void;
  #onChange?: (name: string, binding: string) => void;

  constructor(root: HTMLElement, onChange?: (name: string, binding: string) => void) {
    this.root = root;
    this.#onChange = onChange;

    for (const el of root.querySelectorAll<HTMLElement>("[data-rad-keybind]")) {
      el.addEventListener("click", () => this.#begin(el));
    }

    this.#onKey = (e) => this.#capture(e);
    // Capture phase: a bare letter would otherwise reach whatever has focus first.
    window.addEventListener("keydown", this.#onKey, true);
  }

  get isListening(): boolean {
    return this.#listening !== null;
  }

  destroy(): void {
    window.removeEventListener("keydown", this.#onKey, true);
  }

  /** The combination a KeyboardEvent describes, or null for a bare modifier. */
  static describe(e: KeyboardEvent): string | null {
    const parts: string[] = [];
    if (e.ctrlKey) parts.push("Ctrl");
    if (e.altKey) parts.push("Alt");
    if (e.shiftKey) parts.push("Shift");
    if (e.metaKey) parts.push("Meta");
    if (["Control", "Alt", "Shift", "Meta"].includes(e.key)) return null;
    parts.push(e.key.length === 1 ? e.key.toUpperCase() : e.key);
    return parts.join(" + ");
  }

  #begin(el: HTMLElement): void {
    if (this.#listening) this.#listening.el.classList.remove("is-listening");
    this.#listening = { el, previous: el.textContent ?? "" };
    el.classList.add("is-listening");
    el.textContent = "Press a key…";
  }

  #capture(e: KeyboardEvent): void {
    const listening = this.#listening;
    if (!listening) return;
    e.preventDefault();
    e.stopPropagation();

    const { el, previous } = listening;
    if (e.key === "Escape") {
      el.textContent = previous;
    } else if (e.key === "Delete" || e.key === "Backspace") {
      el.textContent = KeybindCapture.UNSET;
    } else {
      const binding = KeybindCapture.describe(e);
      // A bare modifier is not a binding; keep listening for the rest of it.
      if (binding === null) return;
      el.textContent = binding;
    }

    el.classList.remove("is-listening");
    this.#listening = null;
    this.#onChange?.(el.dataset.radKeybind ?? "", el.textContent ?? "");
  }
}
