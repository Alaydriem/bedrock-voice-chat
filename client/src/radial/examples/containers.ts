import { IconBinding } from "$radial/bindings/IconBinding";
import { AnimationLoop } from "$radial/core/canvas/AnimationLoop";
import { Diagnostics } from "$radial/core/controllers/Diagnostics";
import { DragReorder } from "$radial/core/controllers/DragReorder";
import { FormControls } from "$radial/core/controllers/FormControls";
import { KvGridView } from "$radial/core/controllers/KvGridView";
import { LogConsole, type LogLevel } from "$radial/core/controllers/LogConsole";
import { MENU_DIVIDER, Menu } from "$radial/core/controllers/Menu";
import { SelectControl } from "$radial/core/controllers/SelectControl";
import { TableController } from "$radial/core/controllers/TableController";
import { Toast } from "$radial/core/controllers/Toast";
import { Gallery } from "./gallery";

Gallery.boot();

const page = document.querySelector<HTMLElement>(".g-wrap");
const menuEl = document.querySelector<HTMLElement>("[data-rad-menu]");
if (!page || !menuEl) throw new Error("containers: page scaffold missing");

const menu = new Menu(menuEl, page);
new FormControls(page);
SelectControl.bindAll(page, menu);

/* ---------------------------------------------------------------- the table */

interface Sound {
  name: string;
  length: string;
  size: string;
  bytes: number;
  seconds: number;
}

const SOUNDS: Sound[] = [
  { name: "airhorn.ogg", length: "0:02", size: "38 KB", bytes: 38_000, seconds: 2 },
  { name: "applause.ogg", length: "0:06", size: "142 KB", bytes: 142_000, seconds: 6 },
  { name: "creeper-hiss.ogg", length: "0:03", size: "61 KB", bytes: 61_000, seconds: 3 },
  { name: "drumroll.ogg", length: "0:04", size: "96 KB", bytes: 96_000, seconds: 4 },
  { name: "ender-portal.ogg", length: "0:08", size: "188 KB", bytes: 188_000, seconds: 8 },
  { name: "fanfare.ogg", length: "0:05", size: "120 KB", bytes: 120_000, seconds: 5 },
  { name: "rimshot.ogg", length: "0:01", size: "22 KB", bytes: 22_000, seconds: 1 },
  { name: "sad-trombone.ogg", length: "0:03", size: "70 KB", bytes: 70_000, seconds: 3 },
  { name: "villager-hmm.ogg", length: "0:01", size: "19 KB", bytes: 19_000, seconds: 1 },
  { name: "wither-spawn.ogg", length: "0:11", size: "264 KB", bytes: 264_000, seconds: 11 },
  { name: "xp-orb.ogg", length: "0:01", size: "14 KB", bytes: 14_000, seconds: 1 },
  { name: "zombie-groan.ogg", length: "0:04", size: "88 KB", bytes: 88_000, seconds: 4 },
];

const q = <T extends HTMLElement>(sel: string): T => {
  const el = document.querySelector<T>(sel);
  if (!el) throw new Error(`containers: missing ${sel}`);
  return el;
};

const body = q<HTMLElement>("[data-lib-body]");
const soundId = (s: Sound) => `snd_${s.name.replace(/\.[a-z]+$/, "").replace(/-/g, "_")}`;
const escape = (text: string) => {
  const div = document.createElement("div");
  div.textContent = text;
  return div.innerHTML;
};

let rows = [...SOUNDS];
// Mirrored out of the last view so the prev/next buttons do not have to read the
// current page back out of the DOM.
let pageIndex = 0;

const table = new TableController<Sound>({
  rows,
  id: (s) => s.name,
  search: (s) => s.name,
  pageSize: 6,
  columns: [
    { key: "name", sortBy: (s) => s.name },
    { key: "length", sortBy: (s) => s.seconds },
    { key: "size", sortBy: (s) => s.bytes },
  ],
  onRender: (view) => {
    pageIndex = view.pageIndex;
    body.innerHTML = view.page.length
      ? view.page
          .map((s) => {
            const id = soundId(s);
            const selected = view.selected.has(s.name);
            return (
              `<tr class="${selected ? "is-selected" : ""}">` +
              `<td class="rad-select-cell"><button class="rad-checkbox rad-checkbox--compact" role="checkbox" ` +
              `aria-checked="${selected}" data-lib-row="${escape(s.name)}" aria-label="Select ${escape(s.name)}">` +
              '<span class="rad-checkbox__box" data-rad-icon="check"></span></button></td>' +
              `<td><span class="rad-table__name">${escape(s.name)}</span></td>` +
              `<td class="rad-num">${s.length}</td><td class="rad-num">${s.size}</td>` +
              `<td class="rad-num rad-table__id">${id}</td>` +
              '<td class="rad-table__actions"><span class="rad-row-actions">' +
              `<span class="rad-tip" data-rad-tip="Preview locally"><button class="rad-kebab" aria-label="Play ${escape(s.name)}"><span data-rad-icon="play"></span></button></span>` +
              `<span class="rad-tip" data-rad-tip="Copy sound id"><button class="rad-kebab" data-rad-copy="${id}" aria-label="Copy id"><span data-rad-icon="copy"></span></button></span>` +
              `<button class="rad-kebab" data-lib-menu="${escape(s.name)}" aria-label="Actions"><span data-rad-icon="kebab"></span></button>` +
              "</span></td></tr>"
            );
          })
          .join("")
      : '<tr><td colspan="6" class="rad-table__nomatch">No sounds match that.</td></tr>';

    q("[data-lib-count]").textContent = `${view.matching} sound${view.matching === 1 ? "" : "s"}`;

    const pages = ['<button data-lib-page="prev"' + (view.pageIndex === 0 ? " disabled" : "") + ">‹</button>"];
    for (let i = 0; i < view.pageCount; i++) {
      pages.push(`<button class="${i === view.pageIndex ? "is-on" : ""}" data-lib-page="${i}">${i + 1}</button>`);
    }
    pages.push(
      '<button data-lib-page="next"' + (view.pageIndex >= view.pageCount - 1 ? " disabled" : "") + ">›</button>",
    );
    q("[data-lib-pages]").innerHTML = pages.join("");

    const selectAll = q("[data-lib-selall]");
    selectAll.setAttribute("aria-checked", String(view.allSelected));

    const bar = q("[data-lib-selbar]");
    bar.hidden = view.selected.size === 0;
    q("[data-lib-selcount]").textContent = String(view.selected.size);

    for (const th of document.querySelectorAll<HTMLElement>("[data-lib-sort]")) {
      const sorted = th.dataset.libSort === view.sortKey;
      th.classList.toggle("is-sorted", sorted);
      const caret = th.querySelector(".rad-caret");
      if (caret && sorted) caret.textContent = view.sortAscending ? "▲" : "▼";
    }

    IconBinding.sync(body);
  },
});

table.render();

// One delegated listener rather than re-binding after every render.
document.addEventListener("click", (e) => {
  const target = e.target as HTMLElement;

  const row = target.closest<HTMLElement>("[data-lib-row]");
  if (row) {
    table.toggleSelected(row.dataset.libRow ?? "");
    return;
  }
  if (target.closest("[data-lib-selall]")) {
    table.toggleAll();
    return;
  }
  if (target.closest("[data-lib-clear]")) {
    table.clearSelection();
    return;
  }
  if (target.closest("[data-lib-export]")) {
    Toast.show(`Export ${table.selected.size} sounds`);
    return;
  }
  if (target.closest("[data-lib-delete]")) {
    const gone = new Set(table.selected);
    rows = rows.filter((s) => !gone.has(s.name));
    table.setRows(rows);
    table.clearSelection();
    Toast.show("Deleted");
    return;
  }

  const pageButton = target.closest<HTMLElement>("[data-lib-page]");
  if (pageButton) {
    const value = pageButton.dataset.libPage ?? "";
    if (value === "prev") table.goToPage(pageIndex - 1);
    else if (value === "next") table.goToPage(pageIndex + 1);
    else table.goToPage(Number(value));
    return;
  }

  const th = target.closest<HTMLElement>("[data-lib-sort]");
  if (th) {
    table.sort(th.dataset.libSort ?? "");
    return;
  }

  const rowMenu = target.closest<HTMLElement>("[data-lib-menu]");
  if (rowMenu) {
    menu.open(
      rowMenu,
      [
        { label: "Play" },
        { label: "Rename…" },
        MENU_DIVIDER,
        { label: "Show in folder" },
        MENU_DIVIDER,
        { label: "Delete", danger: true },
      ],
      (item) => Toast.show(`${item.label} — ${rowMenu.dataset.libMenu}`),
    );
  }
});

q<HTMLInputElement>("[data-lib-search]").addEventListener("input", (e) => {
  table.search((e.currentTarget as HTMLInputElement).value);
});

/* ---------------------------------------------------------------- reorder */

const reorder = document.querySelector<HTMLElement>("[data-reorder]");
if (reorder) {
  new DragReorder(reorder, (order) => Toast.show(`Order saved — ${order.join(", ")}`));
}

/* ---------------------------------------------------------------- log console */

const logHost = document.querySelector<HTMLElement>("[data-log]");
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

  document.querySelector("[data-log-clear]")?.addEventListener("click", () => log.clear());
  document.querySelector("[data-log-copy]")?.addEventListener("click", () => {
    void navigator.clipboard?.writeText(log.toText()).catch(() => {});
    Toast.show("Copied — the proxy log");
  });
  const filter = document.querySelector<HTMLElement>("[data-log-filter]");
  if (filter) {
    new SelectControl(filter, menu, ["All", "Info", "Warnings", "Errors"], (value) => {
      const map: Record<string, LogLevel | "all"> = {
        All: "all",
        Info: "info",
        Warnings: "warn",
        Errors: "err",
      };
      log.setFilter(map[value] ?? "all");
    });
  }
}

/* ---------------------------------------------------------------- diagnostics grid */

const kvHost = document.querySelector<HTMLElement>("[data-kv]");
if (kvHost) {
  const view = new KvGridView(kvHost);
  let rtt = 41;
  let uptime = 2531;
  let last = 0;
  AnimationLoop.shared().add((t) => {
    if (t - last < 1000) return;
    last = t;
    uptime++;
    rtt = Math.max(9, Math.round(rtt * 0.45 + (34 + Math.random() * 8) * 0.55));
    view.update(
      Diagnostics.groups({
        rtt,
        lossPercent: Number((Math.random() * 0.5).toFixed(1)),
        jitterMs: Math.round(38 + Math.random() * 10),
        jitterDrops: 0,
        datagramsIn: Math.round(46 + Math.random() * 5),
        datagramsOut: Math.round(48 + Math.random() * 4),
        inputDevice: "Focusrite Scarlett 2i2",
        inputRate: 48000,
        outputDevice: "Sennheiser HD 560S",
        outputRate: 48000,
        quicPort: 443,
        protocol: "1.3.0",
        rangeMetres: 80,
        falloff: "inverse-square",
        server: "bvc.alaydriem.com",
        uptimeSeconds: uptime,
        reconnecting: false,
        muted: false,
        noiseGate: "Open",
        deafened: false,
        pttIdle: false,
        mutedOthers: 0,
        visiblePlayers: 4,
      }),
    );
  });
}
