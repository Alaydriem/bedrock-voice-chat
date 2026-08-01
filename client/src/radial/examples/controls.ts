import { FormControls } from "$radial/core/controllers/FormControls";
import { KeybindCapture } from "$radial/core/controllers/KeybindCapture";
import { MENU_DIVIDER, Menu } from "$radial/core/controllers/Menu";
import { SelectControl } from "$radial/core/controllers/SelectControl";
import { Toast } from "$radial/core/controllers/Toast";
import { Gallery } from "./gallery";

Gallery.boot();

const page = document.querySelector<HTMLElement>(".g-wrap");
const menuEl = document.querySelector<HTMLElement>("[data-rad-menu]");
if (!page || !menuEl) throw new Error("controls: page scaffold missing");

// The menu positions itself inside whatever it is told is the frame. On these pages
// that is the document column rather than an app window.
const menu = new Menu(menuEl, page);

new FormControls(page, {
  onToggle: (name, on) => Toast.show(`${name} → ${on ? "on" : "off"}`),
  onSegment: (group, value) => Toast.show(`${group} → ${value}`),
  onRadio: (group, value) => Toast.show(`${group} → ${value}`),
  onStep: (name, value) => Toast.show(`${name} → ${value} ms`),
});

SelectControl.bindAll(page, menu, (value) => Toast.show(`Selected ${value}`));

new KeybindCapture(page, (name, binding) => Toast.show(`${name} → ${binding}`));

/* A table row menu: dividers, and a single destructive item last. */
document.querySelector<HTMLElement>("[data-rad-menu-demo]")?.addEventListener("click", (e) => {
  menu.open(
    e.currentTarget as HTMLElement,
    [
      { label: "Export as BWAV", hint: "multitrack" },
      { label: "Export as MP4 / Opus", hint: "mixdown" },
      { label: "Export tracks…", hint: "choose" },
      MENU_DIVIDER,
      { label: "Show in folder" },
      { label: "Rename…" },
      MENU_DIVIDER,
      { label: "Delete", danger: true },
    ],
    (item) => Toast.show(item.label),
  );
});

/* A split button's caret: the same component, no ticks. */
document.querySelector<HTMLElement>("[data-rad-split-demo]")?.addEventListener("click", (e) => {
  menu.open(
    e.currentTarget as HTMLElement,
    [
      { label: "Export as BWAV", hint: "multitrack" },
      { label: "Export as WAV", hint: "per track" },
      { label: "Export as MP4 / Opus", hint: "mixdown" },
    ],
    (item) => Toast.show(item.label),
  );
});

/* ---- the address field ----
 * Fixed-height line, so the layout does not jump between the three states — and a
 * deliberate pause on "resolving", because a lookup that appears instant teaches people
 * that a slow one is broken.
 */
const HOSTNAME = /^[a-z0-9.-]+\.[a-z]{2,}$/i;
const resolveInput = document.querySelector<HTMLInputElement>("[data-resolve-input]");
const resolveLine = document.querySelector<HTMLElement>("[data-resolve]");
let resolveTimer: ReturnType<typeof setTimeout> | undefined;

resolveInput?.addEventListener("input", () => {
  if (!resolveLine) return;
  resolveLine.className = "rad-resolve";
  resolveLine.textContent = "○ Resolving";
  clearTimeout(resolveTimer);
  const value = resolveInput.value.trim();
  resolveTimer = setTimeout(() => {
    const ok = HOSTNAME.test(value);
    resolveLine.className = `rad-resolve rad-resolve--${ok ? "ok" : "bad"}`;
    resolveLine.textContent = ok
      ? `● Resolved · ${28 + Math.floor(Math.random() * 40)} ms`
      : "✕ Nothing at that address";
  }, 700);
});
