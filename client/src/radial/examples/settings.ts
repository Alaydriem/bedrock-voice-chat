import { IconBinding } from "$radial/bindings/IconBinding";
import type { LevelMeterBinding } from "$radial/bindings/LevelMeterBinding";
import { AnimationLoop } from "$radial/core/canvas/AnimationLoop";
import { Conditional } from "$radial/core/controllers/Conditional";
import { FormControls } from "$radial/core/controllers/FormControls";
import { KeybindCapture } from "$radial/core/controllers/KeybindCapture";
import { LogConsole, type LogLevel } from "$radial/core/controllers/LogConsole";
import { MENU_DIVIDER, Menu } from "$radial/core/controllers/Menu";
import { Modal } from "$radial/core/controllers/Modal";
import { SelectControl } from "$radial/core/controllers/SelectControl";
import { TableController } from "$radial/core/controllers/TableController";
import { Toast } from "$radial/core/controllers/Toast";
import { TypedConfirm } from "$radial/core/controllers/TypedConfirm";
import { SyntheticLevelSource } from "$radial/core/sources/SyntheticLevelSource";
import { Gallery } from "./gallery";

const mounted = Gallery.boot();

const frame = Gallery.requireElement("set-frame");

const q = <T extends HTMLElement>(sel: string): T | null => frame.querySelector<T>(sel);
const need = <T extends HTMLElement>(sel: string): T => {
  const el = q<T>(sel);
  if (!el) throw new Error(`settings: missing ${sel}`);
  return el;
};

const menu = new Menu(need("[data-rad-menu]"), frame);
const modal = new Modal(frame);
const conditional = new Conditional(frame);

/* ---------------------------------------------------------------- navigation */

const NAV: readonly (readonly [string, readonly (readonly [string, string, string?])[]])[] = [
  ["", [
    ["account", "Account"],
    ["audio", "Audio settings"],
    ["recordings", "Recordings"],
    ["keybinds", "Keybinds"],
    ["ws", "WebSocket server"],
    ["about", "About", "1"],
  ]],
  ["Minecraft Bedrock", [["proxy", "Proxy Connect"], ["realms", "Realms Connect"]]],
];

const TITLES: Record<string, string> = {
  account: "Account",
  audio: "Audio settings",
  recordings: "Recordings",
  keybinds: "Keybinds",
  ws: "WebSocket server",
  proxy: "Proxy Connect",
  realms: "Realms Connect",
  about: "About",
};

let pane = "ws";
let mobileLevel: "list" | "detail" = "list";

function renderNav(): void {
  need("[data-nav]").innerHTML = NAV.map(
    ([group, items]) =>
      (group ? `<div class="rad-nav-group">${group}</div>` : "") +
      items
        .map(
          ([id, label, badge]) =>
            `<button class="rad-nav-item ${id === pane ? "is-on" : ""}" data-pane-go="${id}" ` +
            `aria-current="${id === pane ? "page" : "false"}">${label}` +
            (badge ? `<span class="rad-nav-item__badge">${badge}</span>` : "") +
            "</button>",
        )
        .join(""),
  ).join("");

  // The same list, as the phone's first level. One source, so a pane can never exist in
  // one place and not the other.
  need("[data-mobile-list]").innerHTML = NAV.map(
    ([group, items]) =>
      (group ? `<div class="rad-mobile-group">${group}</div>` : "") +
      items
        .map(
          ([id, label, badge]) =>
            `<button class="rad-mobile-row" data-pane-go="${id}">${label}` +
            (badge ? `<span class="rad-mobile-row__badge">${badge}</span>` : "") +
            '<span class="rad-mobile-row__chevron" data-rad-icon="chev"></span></button>',
        )
        .join(""),
  ).join("");

  IconBinding.sync(frame);
}

function goToPane(next: string): void {
  pane = next;
  for (const section of frame.querySelectorAll<HTMLElement>("[data-pane]")) {
    section.hidden = section.dataset.pane !== pane;
  }
  for (const el of frame.querySelectorAll<HTMLElement>("[data-pane-title]")) {
    el.textContent = TITLES[pane] ?? "Settings";
  }
  for (const b of document.querySelectorAll<HTMLElement>("[data-pane-to]")) {
    b.setAttribute("aria-pressed", String(b.dataset.paneTo === pane));
  }
  renderNav();
  setMobileLevel("detail");
  // A pane change is a new page as far as the reader is concerned.
  const body = q<HTMLElement>(".rad-settings-body");
  if (body) body.scrollTop = 0;
}

function setMobileLevel(level: "list" | "detail"): void {
  mobileLevel = level;
  frame.classList.toggle("is-list", level === "list");
  const title = q("[data-mobile-title]");
  if (title) title.textContent = level === "list" ? "Settings" : (TITLES[pane] ?? "Settings");
  const back = q("[data-mobile-back] [data-rad-icon]");
  // At the top level the back button closes settings, so it is an X rather than an
  // arrow: the arrow would promise a screen that is not there.
  if (back) new IconBinding(back, level === "list" ? "close" : "back");
}

document.addEventListener("click", (e) => {
  const to = (e.target as HTMLElement).closest<HTMLElement>("[data-pane-to],[data-pane-go]");
  if (to) goToPane(to.dataset.paneTo ?? to.dataset.paneGo ?? pane);
});

need("[data-mobile-back]").addEventListener("click", () => {
  if (mobileLevel === "detail") setMobileLevel("list");
  else Toast.show("Closes settings, back to the dashboard");
});

/* ---------------------------------------------------------------- controls */

new FormControls(frame, {
  onToggle: (name, on) => {
    if (name !== "ws") return;
    // The chip and the conditional rows are both derived from the toggle, so the state
    // is stated once and the page cannot contradict itself.
    conditional.state = on ? "realm ws" : "realm";
    const chip = q("[data-ws-state]");
    if (chip) {
      chip.textContent = on ? "Listening" : "Stopped";
      chip.className = `rad-status-chip rad-status-chip--${on ? "ok" : "muted"}`;
    }
  },
  onSegment: (group, value) => Toast.show(`${group} → ${value}`),
});

SelectControl.bindAll(frame, menu, (value, el) => {
  if (el.dataset.logFilter !== undefined) return;
  Toast.show(`Selected ${value}`);
});

new KeybindCapture(frame, (name, binding) => Toast.show(`${name} → ${binding}`));

const gate = q<HTMLInputElement>("[data-gate]");
const gateValue = q("[data-gate-value]");
gate?.addEventListener("input", () => {
  if (gateValue) gateValue.textContent = `${gate.value} dBFS`;
});

/* ---- the mic test: a real meter, driven only while testing ---- */
const testMeter = mounted.byId.get("test-meter") as LevelMeterBinding | undefined;
const testButton = q("[data-test-mic]");
let testing = false;
const testSource = new SyntheticLevelSource({ phase: 0.4 });
testButton?.addEventListener("click", () => {
  testing = !testing;
  testMeter?.setSource(testing ? testSource : null);
  testButton.classList.toggle("rad-btn--primary", testing);
  testButton.innerHTML = testing
    ? '<span data-rad-icon="stop"></span> Stop'
    : '<span data-rad-icon="play"></span> Test';
  IconBinding.sync(testButton);
});

/* ---------------------------------------------------------------- realms */

frame.addEventListener("click", (e) => {
  const use = (e.target as HTMLElement).closest<HTMLElement>("[data-use-realm]");
  if (!use) return;
  for (const b of frame.querySelectorAll<HTMLElement>("[data-use-realm]")) {
    const on = b === use;
    b.setAttribute("aria-pressed", String(on));
    b.textContent = on ? "In use" : "Use";
  }
  const backend = q("[data-proxy-backend]");
  if (backend) backend.textContent = use.dataset.useRealm ?? "";
  Toast.show(`Proxy backend → ${use.dataset.useRealm}`);
});

/* ---------------------------------------------------------------- the proxy log */

const logHost = q("[data-log]");
if (logHost) {
  const log = new LogConsole(logHost);
  const LINES: readonly (readonly [LogLevel, string])[] = [
    ["info", "listening on 127.0.0.1:19132"],
    ["info", "handshake from 127.0.0.1:54122"],
    ["info", "backend RakNet ping ok  protocol=800  version=1.21.100"],
    ["info", "advertising version 1.21.100 (mirrored from backend)"],
    ["info", "tunnel established  client=Alaydriem"],
    ["info", "position batch forwarded  n=20  rate=20/s"],
    ["warn", "position batch late by 118 ms"],
    ["warn", "backend MTU probe failed, falling back to 1400"],
    ["err", "xbl token refresh returned 429, retrying in 30 s"],
    ["info", "xbl token refreshed"],
    ["info", "realm address resolved  ttl=300 s"],
  ];
  let i = 0;
  let last = 0;
  AnimationLoop.shared().add((t) => {
    if (t - last < 2200) return;
    last = t;
    const [level, message] = LINES[i % LINES.length];
    i++;
    log.push(level, message);
  });

  q("[data-log-clear]")?.addEventListener("click", () => log.clear());
  q("[data-log-copy]")?.addEventListener("click", () => {
    void navigator.clipboard?.writeText(log.toText()).catch(() => {});
    Toast.show("Copied — the proxy log");
  });
  const filter = q("[data-log-filter]");
  if (filter) {
    new SelectControl(filter, menu, ["All", "Info", "Warnings", "Errors"], (value) => {
      const map: Record<string, LogLevel | "all"> = { All: "all", Info: "info", Warnings: "warn", Errors: "err" };
      log.setFilter(map[value] ?? "all");
    });
  }
}

/* ---------------------------------------------------------------- recordings */

interface Recording {
  name: string;
  recorded: string;
  length: string;
  tracks: number;
  size: string;
  bytes: number;
}

let RECORDINGS: Recording[] = [
  { name: "Nether run — 2026-07-28", recorded: "2026-07-28 21:14", length: "1:42:08", tracks: 5, size: "412 MB", bytes: 412e6 },
  { name: "Ocean monument raid", recorded: "2026-07-26 19:02", length: "58:31", tracks: 4, size: "221 MB", bytes: 221e6 },
  { name: "Build stream — ep. 12", recorded: "2026-07-24 17:40", length: "3:05:52", tracks: 7, size: "1.1 GB", bytes: 1.1e9 },
  { name: "Quick test", recorded: "2026-07-22 11:09", length: "2:17", tracks: 2, size: "9 MB", bytes: 9e6 },
];

let forceEmpty = false;
const escape = (text: string) => {
  const div = document.createElement("div");
  div.textContent = text;
  return div.innerHTML;
};

const recBody = q("[data-rec-body]");
const recTable = q("[data-rec-table]");
const recEmpty = q("[data-rec-empty]");

const recordings = new TableController<Recording>({
  rows: RECORDINGS,
  id: (r) => r.name,
  search: (r) => r.name,
  columns: [
    { key: "name", sortBy: (r) => r.name },
    { key: "date", sortBy: (r) => r.recorded },
    { key: "size", sortBy: (r) => r.bytes },
  ],
  onRender: (view) => {
    if (!recBody || !recTable || !recEmpty) return;
    // Empty is a different screen, not a table with no rows in it.
    const empty = forceEmpty || view.matching === 0;
    recTable.hidden = empty;
    recEmpty.hidden = !empty;

    recBody.innerHTML = view.page
      .map((r) => {
        const selected = view.selected.has(r.name);
        return (
          `<tr class="${selected ? "is-selected" : ""}">` +
          `<td class="rad-select-cell"><button class="rad-checkbox rad-checkbox--compact" role="checkbox" ` +
          `aria-checked="${selected}" data-rec-row="${escape(r.name)}" aria-label="Select ${escape(r.name)}">` +
          '<span class="rad-checkbox__box" data-rad-icon="check"></span></button></td>' +
          `<td><span class="rad-table__name">${escape(r.name)}</span></td>` +
          `<td class="rad-num">${r.recorded}</td><td class="rad-num">${r.length}</td>` +
          `<td class="rad-num">${r.tracks}</td><td class="rad-num">${r.size}</td>` +
          `<td class="rad-table__actions"><button class="rad-kebab" data-rec-menu="${escape(r.name)}" ` +
          'aria-label="Actions"><span data-rad-icon="kebab"></span></button></td></tr>'
        );
      })
      .join("");

    const selall = q("[data-rec-selall]");
    selall?.setAttribute("aria-checked", String(view.allSelected));
    const bar = q("[data-rec-selbar]");
    if (bar) bar.hidden = view.selected.size === 0;
    const count = q("[data-rec-selcount]");
    if (count) count.textContent = String(view.selected.size);

    const meta = q("[data-rec-meta]");
    if (meta) {
      const total = RECORDINGS.reduce((sum, r) => sum + r.bytes, 0);
      meta.textContent = `${RECORDINGS.length} session${RECORDINGS.length === 1 ? "" : "s"} · ${(total / 1e9).toFixed(1)} GB`;
    }
    const wipeCount = q("[data-wipe-count]");
    if (wipeCount) wipeCount.textContent = `${RECORDINGS.length} sessions`;

    for (const th of frame.querySelectorAll<HTMLElement>("[data-rec-sort]")) {
      const sorted = th.dataset.recSort === view.sortKey;
      th.classList.toggle("is-sorted", sorted);
      const caret = th.querySelector(".rad-caret");
      if (caret && sorted) caret.textContent = view.sortAscending ? "▲" : "▼";
    }

    IconBinding.sync(recBody);
  },
});

let pendingDelete: Recording | null = null;

frame.addEventListener("click", (e) => {
  const target = e.target as HTMLElement;

  const row = target.closest<HTMLElement>("[data-rec-row]");
  if (row) {
    recordings.toggleSelected(row.dataset.recRow ?? "");
    return;
  }
  if (target.closest("[data-rec-selall]")) {
    recordings.toggleAll();
    return;
  }
  if (target.closest("[data-rec-clear]")) {
    recordings.clearSelection();
    return;
  }
  if (target.closest("[data-rec-export]")) {
    Toast.show(`Export ${recordings.selected.size} sessions`);
    return;
  }
  if (target.closest("[data-rec-delete]")) {
    const gone = new Set(recordings.selected);
    RECORDINGS = RECORDINGS.filter((r) => !gone.has(r.name));
    recordings.setRows(RECORDINGS);
    recordings.clearSelection();
    Toast.show("Deleted");
    return;
  }
  const th = target.closest<HTMLElement>("[data-rec-sort]");
  if (th) {
    recordings.sort(th.dataset.recSort ?? "");
    return;
  }
  if (target.closest("[data-toggle-empty]")) {
    forceEmpty = !forceEmpty;
    recordings.render();
    return;
  }

  const rowMenu = target.closest<HTMLElement>("[data-rec-menu]");
  if (rowMenu) {
    const recording = RECORDINGS.find((r) => r.name === rowMenu.dataset.recMenu);
    if (!recording) return;
    menu.open(
      rowMenu,
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
      (item) => {
        if (item.label !== "Delete") {
          Toast.show(`${item.label} — ${recording.name}`);
          return;
        }
        pendingDelete = recording;
        const name = q("[data-delete-name]");
        if (name) name.textContent = recording.name;
        modal.open("delete", rowMenu);
      },
    );
  }
});

q("[data-delete-go]")?.addEventListener("click", () => {
  if (pendingDelete) {
    RECORDINGS = RECORDINGS.filter((r) => r !== pendingDelete);
    recordings.setRows(RECORDINGS);
  }
  modal.close();
  Toast.show("Deleted");
});

const wipeInput = q<HTMLInputElement>("[data-wipe-input]");
const wipeGo = q<HTMLButtonElement>("[data-wipe-go]");
if (wipeInput && wipeGo) {
  const typed = new TypedConfirm(wipeInput, wipeGo, "delete all");
  for (const opener of frame.querySelectorAll<HTMLElement>('[data-rad-modal-open="wipe"]')) {
    opener.addEventListener("click", () => {
      typed.reset();
      typed.focus();
    });
  }
  wipeGo.addEventListener("click", () => {
    RECORDINGS = [];
    recordings.setRows(RECORDINGS);
    recordings.clearSelection();
    forceEmpty = true;
    recordings.render();
    modal.close();
    Toast.show("All recordings deleted");
  });
}

/* ---------------------------------------------------------------- boot */

renderNav();
goToPane("ws");
setMobileLevel("list");
recordings.render();
