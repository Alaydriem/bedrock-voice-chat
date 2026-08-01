import { SyntheticLevelSource } from "../core/sources/SyntheticLevelSource";
import type { Binding } from "./Binding";
import { GlyphBinding } from "./GlyphBinding";
import { IconBinding } from "./IconBinding";
import { LevelMeterBinding } from "./LevelMeterBinding";
import { MarkBinding } from "./MarkBinding";
import { RingBinding, type RingMode } from "./RingBinding";
import { ScopeBinding } from "./ScopeBinding";

/**
 * Wire every Radial binding present in a subtree.
 *
 *   const mounted = Mount.scan(document.body);
 *   ...
 *   mounted.destroy();
 *
 * This is what lets a reference page be plain HTML with one script tag. It is also
 * what an LLM composing a screen should reach for: write the markup, call scan,
 * and the canvases come alive with synthetic data.
 *
 * `data-rad-level` mounts with a synthetic source unless `data-source="none"`, so a
 * page of markup animates before anything real is connected. In the app, construct
 * LevelMeterBinding directly and hand it the real LevelSource.
 */
export class Mount {
  readonly bindings: Binding[] = [];

  /** Indexed by the element's `data-rad-id`, for pages that need a handle back. */
  readonly byId = new Map<string, Binding>();

  static scan(root: ParentNode = document): Mount {
    const mount = new Mount();

    for (const el of root.querySelectorAll<HTMLElement>("[data-rad-icon]")) {
      mount.#add(el, new IconBinding(el));
    }

    for (const canvas of root.querySelectorAll<HTMLCanvasElement>("canvas[data-rad-mark]")) {
      mount.#add(
        canvas,
        new MarkBinding(canvas, {
          cell: Mount.#num(canvas.dataset.cell, 6),
          color: canvas.dataset.color,
          mortar: canvas.dataset.mortar !== "false",
          gain: Mount.#num(canvas.dataset.gain, 1),
          still: canvas.dataset.still === "true",
          bleed: canvas.dataset.bleed !== "false",
        }),
      );
    }

    for (const canvas of root.querySelectorAll<HTMLCanvasElement>("canvas[data-rad-level]")) {
      const source =
        canvas.dataset.source === "none"
          ? undefined
          : new SyntheticLevelSource({
              phase: Mount.#num(canvas.dataset.phase, 0),
              peak: Mount.#num(canvas.dataset.peak, 1),
              gain: Mount.#num(canvas.dataset.gain, 1),
            });
      mount.#add(
        canvas,
        new LevelMeterBinding(canvas, {
          source,
          color: canvas.dataset.color,
          cell: Mount.#num(canvas.dataset.cell, 3),
          onLive: Mount.#liveToggle(canvas),
        }),
      );
    }

    for (const canvas of root.querySelectorAll<HTMLCanvasElement>("canvas[data-rad-ring]")) {
      const mode = canvas.dataset.radRing || "live";
      if (mode === "scope") {
        mount.#add(canvas, new ScopeBinding(canvas, { unit: canvas.dataset.unit }));
        continue;
      }
      // `intro` is deliberately not auto-mounted: a boot sequence needs its own
      // lifecycle, and a page that wanted one would be surprised by a loop it did
      // not start. Construct IntroSequence or Loader.mount directly.
      if (mode === "intro") continue;
      mount.#add(
        canvas,
        new RingBinding(canvas, {
          mode: mode as RingMode,
          scale: Mount.#num(canvas.dataset.scale, 1),
          logoScale: Mount.#num(canvas.dataset.logoScale, 1),
          spin: Mount.#num(canvas.dataset.spin, 0),
        }),
      );
    }

    for (const canvas of root.querySelectorAll<HTMLCanvasElement>("canvas[data-rad-glyph]")) {
      const size = canvas.dataset.size ? Number(canvas.dataset.size) : undefined;
      mount.#add(canvas, new GlyphBinding(canvas, canvas.dataset.radGlyph ?? "", { size }));
    }

    return mount;
  }

  destroy(): void {
    for (const binding of this.bindings) binding.destroy();
    this.bindings.length = 0;
    this.byId.clear();
  }

  #add(el: HTMLElement, binding: Binding): void {
    this.bindings.push(binding);
    const id = el.dataset.radId;
    if (id) this.byId.set(id, binding);
  }

  static #num(raw: string | undefined, fallback: number): number {
    if (raw === undefined) return fallback;
    const n = Number.parseFloat(raw);
    return Number.isFinite(n) ? n : fallback;
  }

  /**
   * A meter inside a player card lights the card. Doing it here rather than in the
   * card means the card has no idea when someone is speaking — the meter is the
   * single place that knows.
   */
  static #liveToggle(canvas: HTMLElement): ((live: boolean) => void) | undefined {
    const card = canvas.closest<HTMLElement>("[data-rad-live-target]");
    if (!card) return undefined;
    return (live) => card.classList.toggle("is-live", live);
  }
}
