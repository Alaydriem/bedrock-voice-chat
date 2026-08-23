import { Surface } from "../core/canvas/Surface";
import { ServerGlyph } from "../core/glyph/ServerGlyph";
import type { Binding } from "./Binding";

export interface GlyphOptions {
  /** Edge length in CSS px. Read from the layout box when omitted. */
  size?: number;
}

/**
 * A server's glyph.
 *
 *   <canvas data-rad-glyph="bvc.example.com"></canvas>
 *
 * Static: drawn once per name and size, never on the animation loop. A rail of
 * servers that repainted every frame would cost more than everything else on the
 * dashboard combined and would look identical.
 */
export class GlyphBinding implements Binding {
  readonly canvas: HTMLCanvasElement;

  #surface: Surface;
  #options: GlyphOptions;
  #name = "";
  #drawnAt = "";

  constructor(canvas: HTMLCanvasElement, name: string, options: GlyphOptions = {}) {
    this.canvas = canvas;
    this.#surface = new Surface(canvas);
    this.#options = options;
    this.name = name;
  }

  get name(): string {
    return this.#name;
  }

  set name(value: string) {
    this.#name = value;
    this.render();
  }

  /** @param prog 0 to 1, for a progressive reveal. */
  render(prog = 1): void {
    const size = this.#options.size ?? this.#measured();
    if (!size) return;
    const key = `${this.#name}@${size}@${prog}`;
    if (key === this.#drawnAt) return;
    this.#drawnAt = key;
    this.#surface.resize(size, size);
    const x = this.#surface.begin();
    ServerGlyph.draw(x, this.#name, size, prog);
  }

  destroy(): void {
    // No loop registration and no subscription of its own; the surface holds a resize
    // subscription that does have to be released.
    this.#surface.destroy();
  }

  #measured(): number {
    const rect = this.canvas.getBoundingClientRect();
    return Math.round(Math.min(rect.width, rect.height));
  }
}
