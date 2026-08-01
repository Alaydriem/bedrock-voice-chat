import type { RingBinding } from "$radial/bindings/RingBinding";
import type { ScopeBinding } from "$radial/bindings/ScopeBinding";
import { Icons } from "$radial/core/icons/Icons";
import { MarkData } from "$radial/core/mark/MarkData";
import { ServerGlyph } from "$radial/core/glyph/ServerGlyph";
import type { RingSource } from "$radial/core/ring/RingSource";
import { SyntheticLevelSource } from "$radial/core/sources/SyntheticLevelSource";
import { AnimationLoop } from "$radial/core/canvas/AnimationLoop";
import { GlyphBinding } from "$radial/bindings/GlyphBinding";
import { Gallery } from "./gallery";

const mounted = Gallery.boot();

/* ---- the icon sheet, built from the set itself so it cannot fall behind ---- */
const sheet = document.querySelector<HTMLElement>("[data-g-icon-sheet]");
const count = document.querySelector<HTMLElement>("[data-g-icon-count]");
if (sheet) {
  sheet.innerHTML = Icons.names()
    .map(
      (name) =>
        `<span class="g-col" style="gap:6px;align-items:center;width:74px">` +
        `<span class="rad-icon-btn" data-rad-icon="${name}" style="pointer-events:none"></span>` +
        `<span class="g-tag" style="letter-spacing:.06em">${name}</span></span>`,
    )
    .join("");
  for (const el of sheet.querySelectorAll<HTMLElement>("[data-rad-icon]")) {
    el.innerHTML = Icons.svg(el.dataset.radIcon as never);
  }
}
if (count) count.textContent = `${Icons.names().length} glyphs, one 24×24 box, 1.9 stroke.`;


/* ---- the full spectrum ----
 * Rendered from MarkData rather than transcribed, so the swatches cannot drift from
 * the mark. Every one of these is a legitimate colour: the semantic tokens are three
 * of them lifted for contrast, not the only three that are allowed.
 */
const spectrum = document.querySelector<HTMLElement>("[data-g-spectrum]");
if (spectrum) {
  spectrum.innerHTML = MarkData.COLUMNS.map(([top, bottom, hex], i) => {
    const rows = bottom - top + 1;
    return (
      '<div class="g-swatch">' +
      `<div class="g-swatch__chip" style="background:${hex}"></div>` +
      '<div class="g-swatch__meta">' +
      `<div class="g-swatch__name">column ${String(i).padStart(2, "0")}</div>` +
      `<div class="g-swatch__value">${hex.toUpperCase()} · ${rows} rows</div>` +
      "</div></div>"
    );
  }).join("");
}
const spectrumNote = document.querySelector<HTMLElement>("[data-g-spectrum-note]");
if (spectrumNote) {
  spectrumNote.textContent =
    `All ${MarkData.COLS} columns, drawn from MarkData itself. Violet through blue, cyan, green, ` +
    "yellow and orange to red — any of them can carry meaning.";
}

/* ---- one glyph per column ----
 * Finds a name that hashes to each column, which demonstrates the full hue range a
 * server list can produce rather than the four that the sample hostnames happen to hit.
 */
const spread = document.querySelector<HTMLElement>("[data-g-glyph-spread]");
if (spread) {
  const names = new Map<number, string>();
  for (let n = 0; names.size < MarkData.COLS && n < 20000; n++) {
    const name = `server-${n}`;
    const { hueIndex } = ServerGlyph.of(name);
    if (!names.has(hueIndex)) names.set(hueIndex, name);
  }
  spread.innerHTML = [...names.entries()]
    .sort((a, b) => a[0] - b[0])
    .map(([, name]) => `<canvas data-rad-glyph="${name}" data-size="34"></canvas>`)
    .join("");
  for (const canvas of spread.querySelectorAll<HTMLCanvasElement>("canvas[data-rad-glyph]")) {
    new GlyphBinding(canvas, canvas.dataset.radGlyph ?? "", { size: 34 });
  }
}

/* ---- the live ring: three voices moving around it ----
 * Placed by angle and volume, tinted by hue. Nothing here knows their names, which is
 * exactly the constraint the real ring works under.
 */
const live = mounted.byId.get("ring-live") as RingBinding | undefined;
if (live) {
  const voices = [
    { source: new SyntheticLevelSource({ phase: 0, peak: 1 }), hue: "#21d8d8", drift: 0.00028 },
    { source: new SyntheticLevelSource({ phase: 1.7, peak: 0.9 }), hue: "#aee236", drift: -0.0002 },
    { source: new SyntheticLevelSource({ phase: 3.4, peak: 0.95 }), hue: "#f67414", drift: 0.00014 },
  ];
  AnimationLoop.shared().add((t) => {
    const sources: RingSource[] = [];
    voices.forEach((v, i) => {
      const level = v.source.at(t);
      if (level <= 0.03) return;
      sources.push({
        angle: -Math.PI / 2 + i * ((Math.PI * 2) / 3) + t * v.drift,
        volume: level,
        hue: v.hue,
      });
    });
    live.setSources(sources);
  });
}

/* ---- the lock ring: one source, acquiring ---- */
const lock = mounted.byId.get("ring-lock") as RingBinding | undefined;
if (lock) {
  AnimationLoop.shared().add((t) => {
    lock.setSources([
      {
        angle: -Math.PI / 2 + 0.5,
        volume: 0.72 + 0.28 * Math.abs(Math.sin(t * 0.0022)),
        hue: "#ad76f7",
      },
    ]);
  });
}

/* ---- the scope ----
 * Scripted through a full incident rather than jittering around a healthy number: the
 * warn and fault thresholds are the whole reason the component has three colours, and a
 * demo that never crosses them never shows two of them.
 *
 * healthy → degrading → bad → recovering → healthy, on a 72 s loop, which is exactly
 * one trip around the ring.
 */
const scope = mounted.byId.get("ring-scope") as ScopeBinding | undefined;
if (scope) {
  const PHASES: readonly { until: number; target: number; jitter: number }[] = [
    { until: 20, target: 36, jitter: 5 },
    { until: 32, target: 72, jitter: 14 },
    { until: 46, target: 118, jitter: 30 },
    { until: 58, target: 64, jitter: 16 },
    { until: 72, target: 34, jitter: 5 },
  ];
  let rtt = 36;
  let second = 0;
  let last = 0;
  AnimationLoop.shared().add((t) => {
    if (t - last < 1000) return;
    last = t;
    second = (second + 1) % 72;
    const phase = PHASES.find((p) => second < p.until) ?? PHASES[PHASES.length - 1];
    const target = phase.target + (Math.random() - 0.4) * phase.jitter;
    // Eased toward the target so neighbouring bars stay related; a scope of independent
    // random values reads as a broken link even when nothing is wrong.
    rtt = Math.max(9, Math.round(rtt * 0.55 + target * 0.45));
    scope.push(rtt);
  });
}
