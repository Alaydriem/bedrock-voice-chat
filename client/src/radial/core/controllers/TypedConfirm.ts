/**
 * A confirmation you have to type out.
 *
 * For the small number of actions that destroy more than one thing and cannot be
 * undone. A button plus a "are you sure?" is enough for one recording; for every
 * recording, the friction has to be proportional to what is being lost.
 *
 * Matching is case-insensitive and trimmed. Making someone fight their own shift key
 * adds no safety, only irritation.
 */
export class TypedConfirm {
  readonly input: HTMLInputElement;
  readonly confirmButton: HTMLButtonElement;
  readonly phrase: string;

  #onInput: () => void;

  constructor(input: HTMLInputElement, confirmButton: HTMLButtonElement, phrase: string) {
    this.input = input;
    this.confirmButton = confirmButton;
    this.phrase = phrase.trim().toLowerCase();
    this.#onInput = () => this.sync();
    input.addEventListener("input", this.#onInput);
    this.reset();
  }

  get matches(): boolean {
    return this.input.value.trim().toLowerCase() === this.phrase;
  }

  sync(): void {
    this.confirmButton.disabled = !this.matches;
  }

  reset(): void {
    this.input.value = "";
    this.sync();
  }

  focus(): void {
    // Deferred past the modal's own opening focus, which would otherwise win.
    setTimeout(() => this.input.focus(), 120);
  }

  destroy(): void {
    this.input.removeEventListener("input", this.#onInput);
  }
}
