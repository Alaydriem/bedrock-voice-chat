import { IconBinding } from "$radial/bindings/IconBinding";
import { Conditional } from "$radial/core/controllers/Conditional";
import { FormControls } from "$radial/core/controllers/FormControls";
import { MENU_DIVIDER, Menu } from "$radial/core/controllers/Menu";
import { Modal } from "$radial/core/controllers/Modal";
import { SelectControl } from "$radial/core/controllers/SelectControl";
import { Sheet } from "$radial/core/controllers/Sheet";
import { Toast } from "$radial/core/controllers/Toast";
import { TypedConfirm } from "$radial/core/controllers/TypedConfirm";
import { Gallery } from "./gallery";

Gallery.boot();

const frame = Gallery.requireElement("ov-frame");
const menuEl = Gallery.require(frame, "[data-rad-menu]");

// Anchored inside the frame, so the flip-near-the-bottom-edge behaviour is measured
// against the app window rather than against the browser viewport.
const menu = new Menu(menuEl, frame);
const modal = new Modal(frame);
const sheet = new Sheet(frame);
new Conditional(frame);
new FormControls(frame);
SelectControl.bindAll(frame, menu, (value) => Toast.show(`Format → ${value}`));

const ROW_ACTIONS = [
  { label: "Export as BWAV", hint: "multitrack" },
  { label: "Export as MP4 / Opus", hint: "mixdown" },
  { label: "Export tracks…", hint: "choose" },
  MENU_DIVIDER,
  { label: "Show in folder" },
  { label: "Rename…" },
  MENU_DIVIDER,
  { label: "Delete", danger: true },
];

function openRowMenu(anchor: HTMLElement): void {
  menu.open(anchor, ROW_ACTIONS, (item) => {
    if (item.label === "Delete") modal.open("confirm", anchor);
    else if (item.label.startsWith("Export tracks")) openExport();
    else Toast.show(item.label);
  });
}

for (const id of ["ov-kebab", "ov-kebab-low"]) {
  document.getElementById(id)?.addEventListener("click", (e) => openRowMenu(e.currentTarget as HTMLElement));
}

/* ---------------------------------------------------------------- the export dialog */

const TRACKS = ["Alaydriem", "Petra", "Juno", "Kestrel", "Moth"];

function openExport(): void {
  const host = frame.querySelector<HTMLElement>("[data-x-tracks]");
  if (host) {
    host.innerHTML = TRACKS.map(
      (name, i) =>
        '<button class="rad-checkbox" role="checkbox" aria-checked="true" data-rad-checkbox="' +
        name +
        '"><span class="rad-checkbox__box" data-rad-icon="check"></span>' +
        `<span class="rad-checkbox__label">${name}</span>` +
        `<span class="rad-checkbox__note">${i === 0 ? "you" : "nearby"}</span></button>`,
    ).join("");
    IconBinding.sync(host);
  }
  const progress = frame.querySelector<HTMLElement>("[data-x-progress]");
  if (progress) {
    progress.style.display = "none";
    const bar = progress.querySelector<HTMLElement>("i");
    if (bar) bar.style.width = "0";
  }
  modal.open("export");
  syncTrackCount();
}

/** The button reports how many tracks it will write, and refuses when that is zero. */
function syncTrackCount(): void {
  const on = frame.querySelectorAll('[data-x-tracks] .rad-checkbox[aria-checked="true"]').length;
  const count = frame.querySelector<HTMLElement>("[data-x-count]");
  if (count) count.textContent = String(on);
  const go = frame.querySelector<HTMLButtonElement>("[data-x-go]");
  if (go) go.disabled = on === 0;
}

frame.addEventListener("click", (e) => {
  if ((e.target as HTMLElement).closest("[data-x-tracks] .rad-checkbox")) syncTrackCount();
});

frame.querySelector<HTMLElement>("[data-x-go]")?.addEventListener("click", () => {
  const progress = frame.querySelector<HTMLElement>("[data-x-progress]");
  const bar = progress?.querySelector<HTMLElement>("i");
  if (!progress || !bar) return;
  progress.style.display = "block";
  let value = 0;
  const timer = setInterval(() => {
    value += 6 + Math.random() * 10;
    bar.style.width = `${Math.min(100, value)}%`;
    if (value < 100) return;
    clearInterval(timer);
    setTimeout(() => {
      const count = frame.querySelector<HTMLElement>("[data-x-count]")?.textContent ?? "0";
      modal.close();
      Toast.show(`Exported ${count} tracks`);
    }, 260);
  }, 90);
});

/* ---------------------------------------------------------------- typed confirmation */

const typedInput = frame.querySelector<HTMLInputElement>("[data-typed-input]");
const typedGo = frame.querySelector<HTMLButtonElement>("[data-typed-go]");
let typed: TypedConfirm | null = null;
if (typedInput && typedGo) {
  typed = new TypedConfirm(typedInput, typedGo, "delete all");
  typedGo.addEventListener("click", () => {
    modal.close();
    Toast.show("All recordings deleted");
  });
}

/* ---------------------------------------------------------------- the demo controls */

for (const button of document.querySelectorAll<HTMLElement>("[data-open]")) {
  button.addEventListener("click", () => {
    switch (button.dataset.open) {
      case "menu":
        openRowMenu(document.getElementById("ov-kebab") as HTMLElement);
        break;
      case "select":
        document.getElementById("ov-select")?.click();
        break;
      case "confirm":
        modal.open("confirm");
        break;
      case "export":
        openExport();
        break;
      case "typed":
        modal.open("typed");
        typed?.reset();
        typed?.focus();
        break;
      case "sheet":
        sheet.open("servers");
        break;
      case "toast":
        Toast.show("Copied — ws://127.0.0.1:8444");
        break;
    }
  });
}
