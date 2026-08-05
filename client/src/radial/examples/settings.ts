import { GlyphBinding } from "$radial/bindings/GlyphBinding";
import { IconBinding } from "$radial/bindings/IconBinding";
import type { LevelMeterBinding } from "$radial/bindings/LevelMeterBinding";
import { AnimationLoop } from "$radial/core/canvas/AnimationLoop";
import { Conditional } from "$radial/core/controllers/Conditional";
import { Cover } from "$radial/core/controllers/Cover";
import { FormControls } from "$radial/core/controllers/FormControls";
import { KeybindCapture } from "$radial/core/controllers/KeybindCapture";
import { LogConsole, type LogLevel } from "$radial/core/controllers/LogConsole";
import { MENU_DIVIDER, Menu, type MenuEntry } from "$radial/core/controllers/Menu";
import { Modal } from "$radial/core/controllers/Modal";
import { SelectControl } from "$radial/core/controllers/SelectControl";
import { TableController } from "$radial/core/controllers/TableController";
import { Toast } from "$radial/core/controllers/Toast";
import { TypedConfirm } from "$radial/core/controllers/TypedConfirm";
import { ServerGlyph } from "$radial/core/glyph/ServerGlyph";
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

const settings = need("[data-settings]");
const menu = new Menu(need("[data-rad-menu]"), frame);
const modal = new Modal(frame);
const conditional = new Conditional(frame);

const escape = (text: string): string => {
  const div = document.createElement("div");
  div.textContent = text;
  return div.innerHTML;
};

/* ---------------------------------------------------------------- the world
 * What settings is looking at from outside itself: the account it is signed in
 * with, what the server says it supports, whether an update is waiting. Every one
 * of these changes what a pane is allowed to claim, and all of them are things the
 * app learns rather than decides — so they are one object with one sync, not
 * conditions scattered through the panes.
 */

type Capability = "on" | "off" | "unknown";

/** What a list of somewhere-to-point is doing. Three screens, not one spinner. */
type ListState = "loading" | "ready" | "failed";

const world = {
  ws: true,
  gate: true,
  xbl: true,
  cap: "on" as Capability,
  update: false,
  libadmin: true,
  proxyUp: true,
  list: "ready" as ListState,
  /**
   * The mobile build, which is not the same switch as a narrow container.
   *
   * A desktop window dragged narrow is still a desktop: it records, it picks its own
   * audio device, it has more than one interface to bind to. Deriving capability from
   * width is how a resized window loses features it still has.
   */
  mobile: false,
};

function syncWorld(): void {
  const tokens = [
    world.ws ? "ws" : "",
    world.gate ? "gate" : "",
    world.xbl ? "xbl" : "xblout",
    world.cap === "off" ? "capoff" : world.cap === "unknown" ? "capunknown" : "",
    world.libadmin ? "libadmin" : "libguest",
    world.proxyUp ? "proxyup" : "",
    world.mobile ? "mobile" : "desktop",
  ].filter(Boolean);
  conditional.state = tokens.join(" ");
  renderNav();
}

/* ---------------------------------------------------------------- navigation */

const NAV: readonly (readonly [string, readonly (readonly [string, string])[]])[] = [
  ["", [
    ["account", "Account"],
    ["audio", "Audio settings"],
    ["players", "Players"],
    ["recordings", "Recordings"],
    ["library", "Audio library"],
    ["keybinds", "Keybinds"],
    ["ws", "WebSocket server"],
    ["about", "About"],
  ]],
  ["Minecraft Bedrock", [["proxy", "Proxy Connect"], ["realms", "Realms Connect"]]],
];

/**
 * Panes the mobile build does not have at all.
 *
 * Recording is not supported there, and a pane that exists only to explain its own
 * absence is worse than no pane: it puts a dead end in the list every time somebody
 * scans it. Keybinds go too — there is no global shortcut to bind on a phone.
 */
const NOT_ON_MOBILE = new Set(["recordings", "keybinds"]);

function navFor(): typeof NAV {
  if (!world.mobile) return NAV;
  return NAV.map(
    ([group, items]) => [group, items.filter(([id]) => !NOT_ON_MOBILE.has(id))] as const,
  ).filter(([, items]) => items.length > 0);
}

const TITLES: Record<string, string> = {
  account: "Account",
  audio: "Audio settings",
  players: "Players",
  recordings: "Recordings",
  library: "Audio library",
  keybinds: "Keybinds",
  ws: "WebSocket server",
  proxy: "Proxy Connect",
  realms: "Realms Connect",
  about: "About",
};

/** Panes whose content is plates or a wide table rather than label-and-control rows. */
const WIDE = new Set(["proxy", "realms", "library", "players"]);

let pane = "account";
let mobileLevel: "list" | "detail" = "list";

/** The only badge in the nav, and it means one thing: an update is waiting. */
function badgeFor(id: string): string {
  return id === "about" && world.update ? '<span class="rad-nav-item__badge">1</span>' : "";
}

function renderNav(): void {
  need("[data-nav]").innerHTML = navFor().map(
    ([group, items]) =>
      (group ? `<div class="rad-nav-group">${group}</div>` : "") +
      items
        .map(
          ([id, label]) =>
            `<button class="rad-nav-item ${id === pane ? "is-on" : ""}" data-pane-go="${id}" ` +
            `aria-current="${id === pane ? "page" : "false"}">${label}` +
            badgeFor(id) +
            "</button>",
        )
        .join(""),
  ).join("");

  // The same list, as the phone's first level. One source, so a pane can never exist in
  // one place and not the other.
  need("[data-mobile-list]").innerHTML = navFor().map(
    ([group, items]) =>
      (group ? `<div class="rad-mobile-group">${group}</div>` : "") +
      items
        .map(
          ([id, label]) =>
            `<button class="rad-mobile-row" data-pane-go="${id}">${label}` +
            badgeFor(id).replace("rad-nav-item__badge", "rad-mobile-row__badge") +
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
  need("[data-measure]").classList.toggle("is-wide", WIDE.has(pane));
  renderNav();
  setMobileLevel("detail");
  // A pane change is a new page as far as the reader is concerned.
  const body = q<HTMLElement>(".rad-settings-body");
  if (body) body.scrollTop = 0;
}

function setMobileLevel(level: "list" | "detail"): void {
  mobileLevel = level;
  settings.classList.toggle("is-list", level === "list");
  const title = q("[data-mobile-title]");
  if (title) title.textContent = level === "list" ? "Settings" : (TITLES[pane] ?? "Settings");
  const back = q("[data-mobile-back] [data-rad-icon]");
  // At the top level the back button closes settings, so it is an X rather than an
  // arrow: the arrow would promise a screen that is not there.
  if (back) new IconBinding(back, level === "list" ? "close" : "back");
}

/** True while the phone's two-level layout is the one on screen. */
function isLevelled(): boolean {
  const backbar = q<HTMLElement>(".rad-backbar");
  return backbar !== null && backbar.offsetParent !== null;
}

document.addEventListener("click", (e) => {
  const to = (e.target as HTMLElement).closest<HTMLElement>("[data-pane-to],[data-pane-go]");
  if (to) goToPane(to.dataset.paneTo ?? to.dataset.paneGo ?? pane);
});

/* ---------------------------------------------------------------- the cover */

const cover = new Cover(frame, {
  // The phone's section list is a level inside settings, so the first back climbs to
  // it and only the second leaves. On desktop the list is never hidden, so there is
  // nothing to climb and back means close.
  onDismiss: () => {
    if (isLevelled() && mobileLevel === "detail") {
      setMobileLevel("list");
      return true;
    }
    return false;
  },
  onOpen: () => syncCoverButtons(true),
  onClose: () => syncCoverButtons(false),
});

function syncCoverButtons(open: boolean): void {
  for (const b of document.querySelectorAll<HTMLElement>("[data-g-cover]")) {
    b.setAttribute("aria-pressed", String((b.dataset.gCover === "open") === open));
  }
}

for (const b of document.querySelectorAll<HTMLElement>("[data-g-cover]")) {
  b.addEventListener("click", () => {
    if (b.dataset.gCover === "open") cover.open("settings");
    else cover.close();
  });
}

need("[data-mobile-back]").addEventListener("click", () => {
  if (mobileLevel === "detail") setMobileLevel("list");
  else cover.close();
});

/* ---------------------------------------------------------------- gallery state */

for (const b of document.querySelectorAll<HTMLElement>("[data-g-state]")) {
  b.addEventListener("click", () => {
    const on = b.getAttribute("aria-pressed") !== "true";
    b.setAttribute("aria-pressed", String(on));
    const key = b.dataset.gState as "xbl" | "update" | "libadmin" | "mobile";
    world[key] = on;
    // A pane the mobile build does not have cannot be the one you are looking at.
    if (key === "mobile" && on && NOT_ON_MOBILE.has(pane)) goToPane("account");
    syncWorld();
    if (key === "libadmin") library.render();
    if (key === "mobile") syncInterface();
  });
}

const LISTS: readonly ListState[] = ["ready", "loading", "failed"];
const LIST_LABEL: Record<ListState, string> = {
  ready: "Lists: loaded",
  loading: "Lists: loading",
  failed: "Lists: failed",
};
for (const b of document.querySelectorAll<HTMLElement>("[data-g-list]")) {
  b.addEventListener("click", () => {
    world.list = LISTS[(LISTS.indexOf(world.list) + 1) % LISTS.length];
    b.textContent = LIST_LABEL[world.list];
    b.setAttribute("aria-pressed", String(world.list === "ready"));
    renderBackends();
    renderRealms();
  });
}

// Three states, so a two-position control cannot express it: the button cycles and
// says which one it is on.
const CAPS: readonly Capability[] = ["on", "unknown", "off"];
const CAP_LABEL: Record<Capability, string> = {
  on: "Bedrock support: on",
  unknown: "Bedrock support: unknown",
  off: "Bedrock support: off",
};
for (const b of document.querySelectorAll<HTMLElement>("[data-g-cap]")) {
  b.addEventListener("click", () => {
    world.cap = CAPS[(CAPS.indexOf(world.cap) + 1) % CAPS.length];
    b.textContent = CAP_LABEL[world.cap];
    b.setAttribute("aria-pressed", String(world.cap === "on"));
    syncWorld();
  });
}

/* ---------------------------------------------------------------- controls */

new FormControls(frame, {
  onToggle: (name, on) => {
    switch (name) {
      case "ws": {
        world.ws = on;
        syncWorld();
        const chip = q("[data-ws-state]");
        if (chip) {
          chip.textContent = on ? "Listening" : "Stopped";
          chip.className = `rad-status-chip rad-status-chip--${on ? "ok" : "muted"}`;
        }
        break;
      }
      case "wslocal":
        syncAddress(on);
        break;
      case "gate":
        world.gate = on;
        syncWorld();
        break;
      case "spatial":
        break;
      case "telemetry":
        Toast.show(on ? "Reports on" : "Reports off — nothing is sent");
        break;
      default:
        break;
    }
  },
  onSegment: (group, value) => {
    if (group === "voice") Toast.show(value === "ptt" ? "Push-to-talk" : "Voice activated");
    if (group === "players") {
      playerScope = value === "all" ? "all" : "adjusted";
      playerPage = 0;
      renderPlayers();
    }
  },
  onCheckbox: () => syncTrackCount(),
});

SelectControl.bindAll(frame, menu, (value, el) => {
  if (el.dataset.logFilter !== undefined) return;
  Toast.show(`Selected ${value}`);
});

new KeybindCapture(frame, (name, binding) => Toast.show(`${name} → ${binding}`));

// The reference pages reach nothing over the network, so a link says where it goes.
frame.addEventListener("click", (e) => {
  const link = (e.target as HTMLElement).closest<HTMLElement>("[data-ext]");
  if (link) Toast.show(link.dataset.ext ?? "");
});

/* ---------------------------------------------------------------- audio */

interface Knob {
  unit: string;
  value: number;
}

const KNOB_DEFAULTS: Record<string, Knob> = {
  open: { unit: "dBFS", value: -40 },
  close: { unit: "dBFS", value: -50 },
  attack: { unit: "ms", value: 4 },
  release: { unit: "ms", value: 120 },
  hold: { unit: "ms", value: 200 },
};

/** Below the open threshold by at least this much, so a steady voice cannot chatter it. */
const GATE_HYSTERESIS = 2;

function knob(name: string): HTMLInputElement | null {
  return q<HTMLInputElement>(`[data-knob="${name}"]`);
}

function paintKnob(name: string): void {
  const input = knob(name);
  const out = q(`[data-knob-value="${name}"]`);
  if (input && out) out.textContent = `${input.value} ${KNOB_DEFAULTS[name].unit}`;
}

/**
 * Keep the two thresholds in the order the label promises.
 *
 * A close threshold above the open one is not a preference, it is a gate that never
 * shuts. Clamped here rather than validated on save, so the pair can never be shown in
 * a state the audio path would refuse.
 */
function clampThresholds(moved: "open" | "close"): void {
  const open = knob("open");
  const close = knob("close");
  if (!open || !close) return;
  if (moved === "open" && Number(close.value) > Number(open.value) - GATE_HYSTERESIS) {
    close.value = String(Number(open.value) - GATE_HYSTERESIS);
    paintKnob("close");
  }
  if (moved === "close" && Number(close.value) > Number(open.value) - GATE_HYSTERESIS) {
    open.value = String(Number(close.value) + GATE_HYSTERESIS);
    paintKnob("open");
  }
}

for (const name of Object.keys(KNOB_DEFAULTS)) {
  knob(name)?.addEventListener("input", () => {
    paintKnob(name);
    if (name === "open" || name === "close") clampThresholds(name);
  });
  paintKnob(name);
}

q("[data-knob-reset]")?.addEventListener("click", () => {
  for (const [name, spec] of Object.entries(KNOB_DEFAULTS)) {
    const input = knob(name);
    if (input) input.value = String(spec.value);
    paintKnob(name);
  }
  Toast.show("Noise gate back to the defaults");
});

const pan = q<HTMLInputElement>("[data-pan]");
const panValue = q("[data-pan-value]");
pan?.addEventListener("input", () => {
  if (panValue) panValue.textContent = `${pan.value}%`;
});

/* ---- the mic test: a real meter, driven only while testing ----
 * Two of them, because the desktop and mobile device cards are different cards. Both
 * are driven together: only one is ever on screen, and leaving the other running would
 * animate a canvas nobody is looking at.
 */
const testMeters = ["test-meter", "test-meter-m"]
  .map((id) => mounted.byId.get(id) as LevelMeterBinding | undefined)
  .filter((meter): meter is LevelMeterBinding => meter !== undefined);
let testing = false;
const testSource = new SyntheticLevelSource({ phase: 0.4 });
for (const button of frame.querySelectorAll<HTMLElement>("[data-test-mic]")) {
  button.addEventListener("click", () => {
    testing = !testing;
    for (const meter of testMeters) meter.setSource(testing ? testSource : null);
    for (const b of frame.querySelectorAll<HTMLElement>("[data-test-mic]")) {
      b.classList.toggle("rad-btn--primary", testing);
      b.innerHTML = testing
        ? '<span data-rad-icon="stop"></span> Stop'
        : '<span data-rad-icon="play"></span> Test';
      IconBinding.sync(b);
    }
  });
}

/* ---------------------------------------------------------------- where it listens
 * Binding and joining are two different addresses and the screen has to hold both.
 * Bound to one interface they agree; bound to everything they do not, and the one you
 * type into Minecraft depends on which device you are typing it on. Saying `0.0.0.0`
 * and leaving it there would be accurate and useless — it is not an address anything
 * can connect to.
 */

const PROXY_PORT = 19132;

interface Interface {
  id: string;
  label: string;
  /** What the proxy binds. */
  bind: string;
  /** What you type into Minecraft on this machine. */
  join: string;
  /** What another device on the network types, when that is a different answer. */
  lan?: string;
}

const INTERFACES: readonly Interface[] = [
  { id: "loopback", label: "127.0.0.1 — this machine only", bind: "127.0.0.1", join: "127.0.0.1" },
  { id: "eth", label: "192.168.1.24 — Ethernet", bind: "192.168.1.24", join: "192.168.1.24" },
  { id: "wifi", label: "192.168.1.31 — Wi-Fi", bind: "192.168.1.31", join: "192.168.1.31" },
  {
    id: "any",
    label: "0.0.0.0 — every interface",
    bind: "0.0.0.0",
    join: "127.0.0.1",
    lan: "192.168.1.24",
  },
];

/** The phone binds everything, because it has no interface worth choosing between. */
const MOBILE_INTERFACE = INTERFACES[3];

let iface = INTERFACES[0];

function currentInterface(): Interface {
  return world.mobile ? MOBILE_INTERFACE : iface;
}

function syncInterface(): void {
  const chosen = currentInterface();

  for (const row of frame.querySelectorAll<HTMLElement>("[data-iface-row]")) {
    const control = row.querySelector<HTMLElement>("[data-iface-control]");
    const note = row.querySelector<HTMLElement>("[data-iface-note]");
    const label = row.querySelector<HTMLElement>(".rad-row__label");
    if (!control || !note || !label) continue;

    if (world.mobile) {
      // A select with one option is a control that cannot be operated. It becomes a
      // statement, because that is what it is.
      label.textContent = "Reachable on this network";
      control.innerHTML = '<span class="rad-status-chip rad-status-chip--ok">Every interface</span>';
      note.textContent =
        `Join on ${MOBILE_INTERFACE.lan}:${PROXY_PORT}.`;
      continue;
    }

    label.textContent = "Listen on";
    control.innerHTML =
      `<button class="rad-select" data-iface data-rad-select="${INTERFACES.map((i) => i.label).join("|")}">` +
      `<span class="rad-select__value">${escape(chosen.label)}</span>` +
      '<span data-rad-icon="chev"></span></button>';
    note.textContent =
      "Which of this machine's addresses the proxy answers on. Loopback is right unless you " +
      "play from a console or a phone on the same network.";
    IconBinding.sync(control);
    const select = control.querySelector<HTMLElement>("[data-iface]");
    if (select) {
      new SelectControl(select, menu, undefined, (value) => {
        iface = INTERFACES.find((i) => i.label === value) ?? INTERFACES[0];
        syncInterface();
      });
    }
  }

  syncJoin();
}

function syncJoin(): void {
  const chosen = currentInterface();
  const address = `${chosen.join}:${PROXY_PORT}`;

  for (const field of frame.querySelectorAll<HTMLInputElement>("[data-join-addr]")) {
    field.value = address;
  }
  for (const copy of frame.querySelectorAll<HTMLElement>("[data-join-copy]")) {
    copy.dataset.radCopy = address;
  }
  for (const warn of frame.querySelectorAll<HTMLElement>("[data-join-warn]")) {
    warn.textContent = address;
  }
  for (const note of frame.querySelectorAll<HTMLElement>("[data-join-note]")) {
    // Only worth saying when the two answers differ. On a single-interface bind the
    // address is the address, and a sentence about other devices is noise.
    note.textContent = chosen.lan
      ? `Add this in Minecraft on this machine. From another device on the network, ${chosen.lan}:${PROXY_PORT}.`
      : "Add this as a server in Minecraft and join it instead.";
  }
}

/* ---------------------------------------------------------------- account */

const LINKED: Record<string, string> = {
  java: "LINKED AS Alaydriem",
  discord: "LINKED AS alaydriem#0",
};

frame.addEventListener("click", (e) => {
  const button = (e.target as HTMLElement).closest<HTMLButtonElement>("[data-link]");
  if (!button) return;
  const which = button.dataset.link ?? "";
  const meta = q(`[data-link-meta="${which}"]`);
  const linked = button.textContent?.trim() === "Link";
  button.textContent = linked ? "Unlink" : "Link";
  if (meta) meta.textContent = linked ? LINKED[which] : "NOT LINKED";
});

/* ---------------------------------------------------------------- players
 * Everyone who has been within earshot, and what you decided about them.
 *
 * The list is written by proximity, not by choice: a player entering earshot is stamped
 * at 100% and unmuted whether or not you ever touch them. So the raw list is hundreds of
 * rows carrying no decision, and the segment defaults to the ones that do. Search serves
 * the other case — a name you remember and want to turn down before they are back.
 */

interface Player {
    gamertag: string;
    gain: number;
    muted: boolean;
    /** Unix ms, or null for an entry written before it was stamped. */
    lastSeen: number | null;
}

const NAMES = [
  "Alaydriem", "Petra", "Juno", "Kestrel", "Moth", "Wren", "Bram", "Sable", "Fen", "Ora",
  "Thistle", "Corvid", "Marlow", "Pike", "Quill", "Rook", "Sorrel", "Tamsin", "Vesper", "Yarrow",
];

let PLAYERS: Player[] = NAMES.map((gamertag, i) => ({
  gamertag,
  // Four with an opinion attached, the rest as proximity left them.
  gain: i === 1 ? 1.45 : i === 4 ? 0.35 : 1,
  muted: i === 6 || i === 11,
  lastSeen: i < 16 ? Date.now() - i * 37 * 60_000 : null,
}));

let playerScope: "adjusted" | "all" = "adjusted";
let playerQuery = "";
let playerPage = 0;

const PLAYERS_PER_PAGE = 6;

/** A row carries a decision when it is muted or no longer at unity gain. */
function isAdjusted(player: Player): boolean {
  return player.muted || Math.abs(player.gain - 1) > 0.001;
}

function ago(lastSeen: number | null): string {
  if (lastSeen === null) return "not seen since this was added";
  const minutes = Math.max(0, Math.round((Date.now() - lastSeen) / 60_000));
  if (minutes < 1) return "just now";
  if (minutes < 60) return `${minutes} min ago`;
  const hours = Math.round(minutes / 60);
  return hours < 24 ? `${hours} h ago` : `${Math.round(hours / 24)} d ago`;
}

function matchingPlayers(): Player[] {
  const query = playerQuery.trim().toLowerCase();
  return PLAYERS.filter((player) => {
    if (playerScope === "adjusted" && !isAdjusted(player)) return false;
    return !query || player.gamertag.toLowerCase().includes(query);
  }).sort((a, b) => {
    // Muted first: "why can't I hear them" is the question this pane answers, and the
    // answer should never be on page two.
    if (a.muted !== b.muted) return a.muted ? -1 : 1;
    return (b.lastSeen ?? 0) - (a.lastSeen ?? 0);
  });
}

function playerRow(player: Player): string {
  const percent = Math.round(player.gain * 100);
  return (
    `<div class="rad-recent-row"${player.muted ? ' data-muted="true"' : ""}>` +
    `<span class="rad-server-id rad-recent-row__id">` +
    `<canvas data-plate-glyph="minecraft:${escape(player.gamertag.toLowerCase())}" width="34" height="34"></canvas>` +
    `</span>` +
    `<span class="rad-recent-row__text">` +
    `<span class="rad-recent-row__name">${escape(player.gamertag)}</span>` +
    `<span class="rad-recent-row__seen">${ago(player.lastSeen)}</span>` +
    `</span>` +
    `<button class="rad-player__mute" aria-pressed="${player.muted}" data-pl-mute="${escape(player.gamertag)}" ` +
    `aria-label="${player.muted ? "Unmute" : "Mute"} ${escape(player.gamertag)}">` +
    `<span data-rad-icon="${player.muted ? "micoff" : "mic"}"></span></button>` +
    `<input class="rad-range" type="range" min="0" max="1.5" step="0.05" value="${player.gain}" ` +
    `${player.muted ? "disabled" : ""} data-pl-gain="${escape(player.gamertag)}" ` +
    `aria-label="Volume for ${escape(player.gamertag)}" />` +
    `<span class="rad-player__percent">${player.muted ? "muted" : `${percent}%`}</span>` +
    `<button class="rad-icon-btn" data-pl-forget="${escape(player.gamertag)}" ` +
    `aria-label="Forget ${escape(player.gamertag)}"><span data-rad-icon="trash"></span></button>` +
    `</div>`
  );
}

function renderPlayers(): void {
  const host = q("[data-pl-list]");
  const card = q("[data-pl-card]");
  const empty = q("[data-pl-empty]");
  if (!host || !card || !empty) return;

  const matching = matchingPlayers();
  const pages = Math.max(1, Math.ceil(matching.length / PLAYERS_PER_PAGE));
  playerPage = Math.min(playerPage, pages - 1);
  const page = matching.slice(
    playerPage * PLAYERS_PER_PAGE,
    playerPage * PLAYERS_PER_PAGE + PLAYERS_PER_PAGE,
  );

  card.hidden = matching.length === 0;
  empty.hidden = matching.length > 0;

  // Three different absences, three different sentences. "No results" for a library you
  // have never added to is the least useful of the three.
  const title = q("[data-pl-empty-title]");
  const note = q("[data-pl-empty-note]");
  if (title && note) {
    if (playerQuery.trim()) {
      title.textContent = "Nobody matches that";
      note.textContent = `No player ${playerScope === "adjusted" ? "you have adjusted " : ""}has that in their name.`;
    } else if (playerScope === "adjusted") {
      title.textContent = "Nothing changed yet";
      note.textContent =
        "Turn somebody up or down from the dashboard and they appear here, so you can change " +
        "your mind after they have gone.";
    } else {
      title.textContent = "Nobody yet";
      note.textContent =
        "Anyone who comes within earshot on a server appears here, so their volume is still " +
        "adjustable once they have wandered off.";
    }
  }

  host.innerHTML = page.map(playerRow).join("");
  hydrate(host);

  const meta = q("[data-pl-meta]");
  if (meta) {
    const adjusted = PLAYERS.filter(isAdjusted).length;
    meta.textContent =
      playerScope === "adjusted"
        ? `${adjusted} adjusted`
        : `${PLAYERS.length} player${PLAYERS.length === 1 ? "" : "s"} · ${adjusted} adjusted`;
  }
  const resetCount = q("[data-pl-reset-count]");
  if (resetCount) {
    const adjusted = PLAYERS.filter(isAdjusted).length;
    resetCount.textContent = `${adjusted} player${adjusted === 1 ? "" : "s"}`;
  }

  const pager = q("[data-pl-pager]");
  if (pager) pager.hidden = pages < 2;
  const count = q("[data-pl-count]");
  if (count) count.textContent = `${matching.length} shown`;
  const pageHost = q("[data-pl-pages]");
  if (pageHost) {
    const buttons = [`<button data-pl-page="prev"${playerPage === 0 ? " disabled" : ""}>‹</button>`];
    for (let i = 0; i < pages; i++) {
      buttons.push(`<button class="${i === playerPage ? "is-on" : ""}" data-pl-page="${i}">${i + 1}</button>`);
    }
    buttons.push(
      `<button data-pl-page="next"${playerPage >= pages - 1 ? " disabled" : ""}>›</button>`,
    );
    pageHost.innerHTML = buttons.join("");
  }
}

q<HTMLInputElement>("[data-pl-search]")?.addEventListener("input", (e) => {
  playerQuery = (e.target as HTMLInputElement).value;
  playerPage = 0;
  renderPlayers();
});

frame.addEventListener("click", (e) => {
  const target = e.target as HTMLElement;

  const mute = target.closest<HTMLElement>("[data-pl-mute]");
  if (mute) {
    const player = PLAYERS.find((p) => p.gamertag === mute.dataset.plMute);
    if (player) player.muted = !player.muted;
    renderPlayers();
    return;
  }
  const forget = target.closest<HTMLElement>("[data-pl-forget]");
  if (forget) {
    PLAYERS = PLAYERS.filter((p) => p.gamertag !== forget.dataset.plForget);
    renderPlayers();
    Toast.show(`Forgot ${forget.dataset.plForget}`);
    return;
  }
  const pageButton = target.closest<HTMLElement>("[data-pl-page]");
  if (pageButton) {
    const value = pageButton.dataset.plPage ?? "";
    if (value === "prev") playerPage = Math.max(0, playerPage - 1);
    else if (value === "next") playerPage += 1;
    else playerPage = Number(value);
    renderPlayers();
  }
});

frame.addEventListener("input", (e) => {
  const slider = (e.target as HTMLElement).closest<HTMLInputElement>("[data-pl-gain]");
  if (!slider) return;
  const player = PLAYERS.find((p) => p.gamertag === slider.dataset.plGain);
  if (!player) return;
  player.gain = Number(slider.value);
  // The readout only, not the whole list: rebuilding mid-drag takes the slider out from
  // under the pointer and the drag stops dead.
  const percent = slider.parentElement?.querySelector<HTMLElement>(".rad-player__percent");
  if (percent) percent.textContent = `${Math.round(player.gain * 100)}%`;
});

q("[data-pl-reset-go]")?.addEventListener("click", () => {
  PLAYERS = PLAYERS.map((player) => ({ ...player, gain: 1, muted: false }));
  modal.close();
  renderPlayers();
  Toast.show("Everyone back to 100%");
});

/* ---------------------------------------------------------------- recordings */

interface Recording {
  name: string;
  recorded: string;
  length: string;
  tracks: number;
  size: string;
  bytes: number;
  players: readonly string[];
  /** False while the session is still being written. */
  exportable: boolean;
}

let RECORDINGS: Recording[] = [
  {
    name: "Nether run — 2026-07-28",
    recorded: "2026-07-28 21:14",
    length: "1:42:08",
    tracks: 5,
    size: "412 MB",
    bytes: 412e6,
    players: ["Alaydriem", "Petra", "Juno", "Kestrel", "Moth"],
    exportable: true,
  },
  {
    name: "Ocean monument raid",
    recorded: "2026-07-26 19:02",
    length: "58:31",
    tracks: 4,
    size: "221 MB",
    bytes: 221e6,
    players: ["Alaydriem", "Petra", "Juno", "Moth"],
    exportable: true,
  },
  {
    name: "Build stream — ep. 12",
    recorded: "2026-07-24 17:40",
    length: "3:05:52",
    tracks: 7,
    size: "1.1 GB",
    bytes: 1.1e9,
    players: ["Alaydriem", "Petra", "Juno", "Kestrel", "Moth", "Wren", "Bram"],
    exportable: true,
  },
  {
    name: "Session in progress",
    recorded: "2026-07-28 22:06",
    length: "07:12",
    tracks: 3,
    size: "31 MB",
    bytes: 31e6,
    players: ["Alaydriem", "Petra", "Wren"],
    exportable: false,
  },
];

let forceEmpty = false;

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
        // A session still being written can be named and deleted but not exported, and
        // says so on the row rather than by a menu item that does nothing.
        const pending = r.exportable
          ? ""
          : ' <span class="rad-status-chip rad-status-chip--live">Recording</span>';
        return (
          `<tr class="${selected ? "is-selected" : ""}">` +
          `<td class="rad-select-cell"><button class="rad-checkbox rad-checkbox--compact" role="checkbox" ` +
          `aria-checked="${selected}" data-rec-row="${escape(r.name)}" aria-label="Select ${escape(r.name)}">` +
          '<span class="rad-checkbox__box" data-rad-icon="check"></span></button></td>' +
          `<td><span class="rad-table__name">${escape(r.name)}</span>${pending}</td>` +
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
let pendingRename: Recording | null = null;

/* ---- export ----
 * One dialog, two shapes. For a single session it lists that session's players,
 * because who to include is a real choice. For a selection it does not: the sessions
 * have different casts, and a checklist that means something different per row is
 * worse than no checklist.
 */

let exportingMany = 0;

function openExport(recording: Recording | null, many = 0): void {
  exportingMany = many;
  const host = q("[data-x-tracks]");
  const card = host?.closest<HTMLElement>(".rad-card");
  if (host && card) {
    card.hidden = many > 0;
    host.innerHTML = (recording?.players ?? [])
      .map(
        (name, i) =>
          '<button class="rad-checkbox" role="checkbox" aria-checked="true" data-rad-checkbox="' +
          escape(name) +
          '"><span class="rad-checkbox__box" data-rad-icon="check"></span>' +
          `<span class="rad-checkbox__label">${escape(name)}</span>` +
          `<span class="rad-checkbox__note">${i === 0 ? "you" : "nearby"}</span></button>`,
      )
      .join("");
    IconBinding.sync(host);
  }
  const progress = q("[data-x-progress]");
  if (progress) {
    progress.style.display = "none";
    const bar = progress.querySelector<HTMLElement>("i");
    if (bar) bar.style.width = "0";
  }
  modal.open("export");
  syncTrackCount();
}

/** The button reports what it will write, and refuses when that is nothing. */
function syncTrackCount(): void {
  const go = q<HTMLButtonElement>("[data-x-go]");
  const count = q("[data-x-count]");
  if (!go || !count) return;
  if (exportingMany > 0) {
    count.textContent = String(exportingMany);
    go.innerHTML = `Export <span data-x-count>${exportingMany}</span> sessions`;
    go.disabled = false;
    return;
  }
  const on = frame.querySelectorAll('[data-x-tracks] .rad-checkbox[aria-checked="true"]').length;
  go.innerHTML = `Export <span data-x-count>${on}</span> tracks`;
  go.disabled = on === 0;
}

q("[data-x-go]")?.addEventListener("click", () => {
  const progress = q("[data-x-progress]");
  const bar = progress?.querySelector<HTMLElement>("i");
  if (!progress || !bar) return;
  progress.style.display = "block";
  let value = 0;
  const timer = window.setInterval(() => {
    value += 6 + Math.random() * 10;
    bar.style.width = `${Math.min(100, value)}%`;
    if (value < 100) return;
    window.clearInterval(timer);
    window.setTimeout(() => {
      const spatial = q('[data-rad-toggle="spatial"]')?.getAttribute("aria-checked") === "true";
      modal.close();
      Toast.show(spatial ? "Exported — spatial mix and tracks" : "Exported — flat tracks");
    }, 260);
  }, 90);
});

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
    openExport(null, recordings.selected.size);
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
        { label: "Export…", hint: `${recording.tracks} tracks`, disabled: !recording.exportable },
        { label: "Rename…" },
        MENU_DIVIDER,
        { label: "Delete", danger: true },
      ],
      (item) => {
        if (item.label === "Export…") {
          openExport(recording);
          return;
        }
        if (item.label === "Rename…") {
          pendingRename = recording;
          const input = q<HTMLInputElement>("[data-rename-input]");
          if (input) input.value = recording.name;
          modal.open("rename", rowMenu);
          input?.focus();
          input?.select();
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

q("[data-rename-go]")?.addEventListener("click", () => {
  const next = q<HTMLInputElement>("[data-rename-input]")?.value.trim();
  if (pendingRename && next) {
    pendingRename.name = next;
    recordings.setRows(RECORDINGS);
  }
  modal.close();
  Toast.show("Renamed");
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

/* ---------------------------------------------------------------- audio library */

interface Sound {
  id: string;
  name: string;
  uploader: string;
  added: string;
  length: string;
  seconds: number;
  size: string;
  bytes: number;
}

let SOUNDS: Sound[] = [
  { id: "snd_village_bell", name: "village-bell.ogg", uploader: "Alaydriem", added: "2026-07-28", length: "0:06", seconds: 6, size: "84 KB", bytes: 84e3 },
  { id: "snd_airhorn", name: "airhorn.ogg", uploader: "Petra", added: "2026-07-27", length: "0:02", seconds: 2, size: "31 KB", bytes: 31e3 },
  { id: "snd_boss_theme", name: "boss-theme.mp3", uploader: "Alaydriem", added: "2026-07-21", length: "3:41", seconds: 221, size: "5.2 MB", bytes: 5.2e6 },
  { id: "snd_creeper_warning", name: "creeper-warning.ogg", uploader: "Juno", added: "2026-07-19", length: "0:04", seconds: 4, size: "58 KB", bytes: 58e3 },
  { id: "snd_tavern_loop", name: "tavern-loop.ogg", uploader: "Kestrel", added: "2026-07-12", length: "2:18", seconds: 138, size: "3.1 MB", bytes: 3.1e6 },
  { id: "snd_victory_fanfare", name: "victory-fanfare.ogg", uploader: "Moth", added: "2026-07-09", length: "0:09", seconds: 9, size: "142 KB", bytes: 142e3 },
  { id: "snd_rain_ambience", name: "rain-ambience.ogg", uploader: "Alaydriem", added: "2026-06-30", length: "5:00", seconds: 300, size: "6.8 MB", bytes: 6.8e6 },
  { id: "snd_door_knock", name: "door-knock.ogg", uploader: "Wren", added: "2026-06-24", length: "0:01", seconds: 1, size: "18 KB", bytes: 18e3 },
];

const libBody = q("[data-lib-body]");
const libTable = q("[data-lib-table]");
const libEmpty = q("[data-lib-empty]");

// Mirrored out of the last view so prev and next do not have to read the current page
// back out of the DOM.
let libPage = 0;

const library = new TableController<Sound>({
  rows: SOUNDS,
  id: (s) => s.id,
  search: (s) => `${s.name} ${s.uploader}`,
  pageSize: 5,
  columns: [
    { key: "name", sortBy: (s) => s.name },
    { key: "date", sortBy: (s) => s.added },
    { key: "length", sortBy: (s) => s.seconds },
    { key: "size", sortBy: (s) => s.bytes },
  ],
  onRender: (view) => {
    if (!libBody || !libTable || !libEmpty) return;
    libPage = view.pageIndex;
    // An empty library and a search that matched nothing are different problems, and
    // only the first one is worth a whole screen.
    const bare = SOUNDS.length === 0;
    libTable.hidden = bare;
    libEmpty.hidden = !bare;

    const columns = world.libadmin ? 8 : 7;
    libBody.innerHTML = view.matching
      ? view.page
          .map((s) => {
            const selected = view.selected.has(s.id);
            return (
              `<tr class="${selected ? "is-selected" : ""}">` +
              (world.libadmin
                ? '<td class="rad-select-cell"><button class="rad-checkbox rad-checkbox--compact" ' +
                  `role="checkbox" aria-checked="${selected}" data-lib-row="${s.id}" ` +
                  `aria-label="Select ${escape(s.name)}">` +
                  '<span class="rad-checkbox__box" data-rad-icon="check"></span></button></td>'
                : "") +
              `<td><span class="rad-table__name">${escape(s.name)}</span></td>` +
              `<td>${escape(s.uploader)}</td>` +
              `<td class="rad-num">${s.added}</td>` +
              `<td class="rad-num">${s.length}</td>` +
              `<td class="rad-num">${s.size}</td>` +
              `<td class="rad-num rad-table__id">${s.id}</td>` +
              '<td class="rad-table__actions"><span class="rad-row-actions">' +
              '<span class="rad-tip" data-rad-tip="Preview locally">' +
              `<button class="rad-kebab" data-lib-play="${s.id}" aria-label="Play ${escape(s.name)}">` +
              '<span data-rad-icon="play"></span></button></span>' +
              '<span class="rad-tip" data-rad-tip="Copy sound id">' +
              `<button class="rad-kebab" data-rad-copy="${s.id}" aria-label="Copy id">` +
              '<span data-rad-icon="copy"></span></button></span>' +
              (world.libadmin
                ? `<button class="rad-kebab" data-lib-menu="${s.id}" aria-label="Actions">` +
                  '<span data-rad-icon="kebab"></span></button>'
                : "") +
              "</span></td></tr>"
            );
          })
          .join("")
      : `<tr><td colspan="${columns}" class="rad-table__nomatch">Nothing matches that.</td></tr>`;

    const meta = q("[data-lib-meta]");
    if (meta) {
      const total = SOUNDS.reduce((sum, s) => sum + s.bytes, 0);
      meta.textContent = `${SOUNDS.length} sound${SOUNDS.length === 1 ? "" : "s"} · ${(total / 1e6).toFixed(1)} MB`;
    }

    const selall = q("[data-lib-selall]");
    selall?.setAttribute("aria-checked", String(view.allSelected));
    const bar = q("[data-lib-selbar]");
    if (bar) bar.hidden = view.selected.size === 0 || !world.libadmin;
    const selcount = q("[data-lib-selcount]");
    if (selcount) selcount.textContent = String(view.selected.size);

    const pager = q("[data-lib-pager]");
    if (pager) pager.hidden = view.pageCount < 2;
    const count = q("[data-lib-count]");
    if (count) count.textContent = `${view.matching} sound${view.matching === 1 ? "" : "s"}`;
    const pageHost = q("[data-lib-pages]");
    if (pageHost) {
      const buttons = ['<button data-lib-page="prev"' + (view.pageIndex === 0 ? " disabled" : "") + ">‹</button>"];
      for (let i = 0; i < view.pageCount; i++) {
        buttons.push(`<button class="${i === view.pageIndex ? "is-on" : ""}" data-lib-page="${i}">${i + 1}</button>`);
      }
      buttons.push(
        '<button data-lib-page="next"' +
          (view.pageIndex >= view.pageCount - 1 ? " disabled" : "") +
          ">›</button>",
      );
      pageHost.innerHTML = buttons.join("");
    }

    for (const th of frame.querySelectorAll<HTMLElement>("[data-lib-sort]")) {
      const sorted = th.dataset.libSort === view.sortKey;
      th.classList.toggle("is-sorted", sorted);
      const caret = th.querySelector(".rad-caret");
      if (caret && sorted) caret.textContent = view.sortAscending ? "▲" : "▼";
    }

    IconBinding.sync(libBody);
  },
});

q<HTMLInputElement>("[data-lib-search]")?.addEventListener("input", (e) => {
  library.search((e.target as HTMLInputElement).value);
});

frame.addEventListener("click", (e) => {
  const target = e.target as HTMLElement;

  const sort = target.closest<HTMLElement>("[data-lib-sort]");
  if (sort) {
    library.sort(sort.dataset.libSort ?? "");
    return;
  }
  const page = target.closest<HTMLElement>("[data-lib-page]");
  if (page) {
    const value = page.dataset.libPage ?? "";
    if (value === "prev") library.goToPage(libPage - 1);
    else if (value === "next") library.goToPage(libPage + 1);
    else library.goToPage(Number(value));
    return;
  }
  const row = target.closest<HTMLElement>("[data-lib-row]");
  if (row) {
    library.toggleSelected(row.dataset.libRow ?? "");
    return;
  }
  if (target.closest("[data-lib-selall]")) {
    library.toggleAll();
    return;
  }
  if (target.closest("[data-lib-clear]")) {
    library.clearSelection();
    return;
  }
  if (target.closest("[data-lib-delete]")) {
    const gone = new Set(library.selected);
    SOUNDS = SOUNDS.filter((s) => !gone.has(s.id));
    library.setRows(SOUNDS);
    library.clearSelection();
    Toast.show("Deleted from the library");
    return;
  }
  // Preview is local. Playing a sound into the world is the jukebox's job and belongs
  // in game, where the person pressing it can hear the room they are playing it into.
  const play = target.closest<HTMLElement>("[data-lib-play]");
  if (play) {
    const sound = SOUNDS.find((s) => s.id === play.dataset.libPlay);
    Toast.show(`Previewing ${sound?.name ?? ""} — only you hear this`);
    return;
  }
  const soundMenu = target.closest<HTMLElement>("[data-lib-menu]");
  if (soundMenu) {
    const sound = SOUNDS.find((s) => s.id === soundMenu.dataset.libMenu);
    if (!sound) return;
    menu.open(
      soundMenu,
      [{ label: "Copy sound id", hint: sound.id }, MENU_DIVIDER, { label: "Delete", danger: true }],
      (item) => {
        if (item.label === "Delete") {
          SOUNDS = SOUNDS.filter((s) => s !== sound);
          library.setRows(SOUNDS);
          Toast.show("Deleted from the library");
          return;
        }
        void navigator.clipboard?.writeText(sound.id).catch(() => {});
        Toast.show(`Copied — ${sound.id}`);
      },
    );
    return;
  }
  if (target.closest("[data-lib-upload]")) Toast.show("Opens a file picker");
});

/* ---------------------------------------------------------------- websocket */

function syncAddress(localOnly: boolean): void {
  const port = q<HTMLInputElement>("[data-ws-port]")?.value.trim() || "8444";
  const address = `ws://${localOnly ? "127.0.0.1" : "0.0.0.0"}:${port}`;
  const field = q<HTMLInputElement>("[data-ws-addr]");
  if (field) field.value = address;
  const copy = q("[data-ws-copy]");
  if (copy) copy.dataset.radCopy = address;
}

q<HTMLInputElement>("[data-ws-port]")?.addEventListener("input", () => {
  syncAddress(q('[data-rad-toggle="wslocal"]')?.getAttribute("aria-checked") === "true");
});

const WS_CLIENTS: readonly (readonly [string, number, number])[] = [
  ["Elgato Stream Deck", 2531, 318],
  ["bvc-cli", 227, 6],
];

function clock(seconds: number): string {
  const h = Math.floor(seconds / 3600);
  const m = Math.floor((seconds % 3600) / 60);
  const s = seconds % 60;
  return [h, m, s].map((n) => String(n).padStart(2, "0")).join(":");
}

function renderClients(elapsed: number): void {
  const host = q("[data-ws-clients]");
  if (!host) return;
  host.innerHTML = WS_CLIENTS.map(
    ([name, since, commands]) =>
      `<tr><td><span class="rad-table__name">${escape(name)}</span></td>` +
      `<td class="rad-num">${clock(since + elapsed)}</td><td class="rad-num">${commands}</td>` +
      '<td class="rad-table__actions"><button class="rad-kebab" aria-label="Actions">' +
      '<span data-rad-icon="kebab"></span></button></td></tr>',
  ).join("");
  IconBinding.sync(host);
}

/* ---------------------------------------------------------------- the plate
 * Shared by Proxy and Realms, because both are asking the same question the server
 * list asks: which of these should this point at. One renderer, so an answer given on
 * one of them cannot look like a different kind of answer on the other.
 */

interface Plate {
  id: string;
  name: string;
  /** The mono line under the name: an address, or a Realm's message of the day. */
  detail: string;
  /** Seeds the derived artwork and the glyph. */
  glyphKey: string;
  chips: readonly (readonly [string, string])[];
  favourite: boolean;
  active: boolean;
  /** False for a Realm that is closed, which cannot be connected to. */
  reachable: boolean;
  /** Operator-supplied entries are theirs, so they cannot be edited or removed here. */
  readonly?: boolean;
}

function plateHtml(p: Plate, i: number): string {
  const hue = ServerGlyph.of(p.glyphKey).hue;
  const chips = p.chips
    .map(([label, kind]) => `<span class="rad-status-chip rad-status-chip--${kind}">${escape(label)}</span>`)
    .join("");
  const action = p.active
    ? `<button class="rad-btn rad-btn--danger" data-plate-stop="${p.id}">Stop</button>`
    : `<button class="rad-btn${p.reachable ? " rad-btn--primary" : ""}" data-plate-go="${p.id}"` +
      `${p.reachable ? "" : " disabled"}>Connect</button>`;
  return (
    `<div class="rad-server rad-rise" style="--d:${i * 50}">` +
    `<span class="rad-server__art rad-server__art--derived" style="--rad-server-hue:${hue}">` +
    `<span class="rad-server-id rad-server__id" style="width:52px;height:52px">` +
    `<canvas data-plate-glyph="${escape(p.glyphKey)}" width="52" height="52"></canvas></span></span>` +
    `<span class="rad-server__body">` +
    `<span class="rad-server__name">${escape(p.name)}</span>` +
    `<span class="rad-server__host">${escape(p.detail)}</span>` +
    `<span class="rad-server__state">${chips}</span>` +
    `</span>` +
    `<span class="rad-server__foot">` +
    `<span style="display:flex;align-items:center;gap:2px">` +
    `<button class="rad-icon-btn rad-fav" data-plate-fav="${p.id}" aria-pressed="${p.favourite}" ` +
    `aria-label="Favourite ${escape(p.name)}"><span data-rad-icon="star"></span></button>` +
    (p.readonly
      ? ""
      : `<button class="rad-kebab" data-plate-menu="${p.id}" aria-label="Actions">` +
        '<span data-rad-icon="kebab"></span></button>') +
    `</span>${action}</span></div>`
  );
}

const addTile = (label: string) =>
  '<button class="rad-server-add" data-plate-add>' +
  '<span data-rad-icon="plus"></span>' +
  `<span class="rad-server-add__label">${label}</span></button>`;

/**
 * Loading, failed and empty are three screens.
 *
 * A skeleton says something is coming and roughly what shape it is. A failure needs a
 * reason and a retry. An empty state needs to say how something gets here. One spinner
 * for all three answers none of the questions — and a grid of tiles shown while the
 * server has already refused is the worst of the four, because every tile in it is an
 * offer that cannot be taken.
 */
interface ListShell {
  failTitle: string;
  failNote: string;
  retry: string;
  emptyTitle: string;
  emptyNote: string;
  /** Rendered under the empty state, where there is something the reader can do. */
  emptyAction?: string;
}

function listHtml(state: ListState, count: number, grid: string, copy: ListShell): string {
  if (state === "loading") {
    return (
      '<div class="rad-card"><div class="rad-card__body" ' +
      'style="display:flex;flex-direction:column;gap:13px">' +
      '<span class="rad-skeleton" style="width:62%"></span>' +
      '<span class="rad-skeleton" style="width:84%"></span>' +
      '<span class="rad-skeleton" style="width:41%"></span>' +
      "</div></div>"
    );
  }
  if (state === "failed") {
    return (
      '<div class="rad-card"><div class="rad-empty" style="padding:30px 20px">' +
      `<span class="rad-empty__title">${escape(copy.failTitle)}</span>` +
      `<span class="rad-empty__note">${copy.failNote}</span>` +
      '<span class="rad-swatchrow" style="justify-content:center">' +
      `<button class="rad-btn rad-btn--primary" data-list-retry>${escape(copy.retry)}</button>` +
      '<button class="rad-btn" data-pane-go="about">See the log</button>' +
      "</span></div></div>"
    );
  }
  if (count === 0) {
    return (
      '<div class="rad-card"><div class="rad-empty" style="padding:26px 20px">' +
      '<canvas data-rad-mark data-cell="4" data-gain="0.35" data-still="true"></canvas>' +
      `<span class="rad-empty__title">${escape(copy.emptyTitle)}</span>` +
      `<span class="rad-empty__note">${copy.emptyNote}</span>` +
      (copy.emptyAction
        ? `<span class="rad-swatchrow" style="justify-content:center">${copy.emptyAction}</span>`
        : "") +
      "</div></div>"
    );
  }
  return `<div class="rad-server-grid">${grid}</div>`;
}

/** Capability refused outranks a fetch that worked: the tiles would be unusable either way. */
function stateFor(): ListState {
  if (world.cap === "off") return "failed";
  return world.list;
}

/** Bindings for markup written after `Mount.scan` ran. */
function hydrate(root: ParentNode): void {
  for (const el of root.querySelectorAll<HTMLElement>("[data-rad-icon]")) new IconBinding(el);
  for (const canvas of root.querySelectorAll<HTMLCanvasElement>("canvas[data-plate-glyph]")) {
    new GlyphBinding(canvas, canvas.dataset.plateGlyph ?? "", { size: canvas.width }).render(1);
  }
}

/* ---------------------------------------------------------------- proxy */

interface Backend {
  id: string;
  name: string;
  host: string;
  port: number;
  favourite: boolean;
  /** Advertised by the BVC server's own config. Read-only, and never stored locally. */
  fromServer: boolean;
}

let BACKENDS: Backend[] = [
  { id: "b1", name: "Alaydriem's SMP", host: "mc.alaydriem.com", port: 19132, favourite: true, fromServer: true },
  { id: "b2", name: "Hearthhold", host: "play.hearthhold.net", port: 19132, favourite: false, fromServer: true },
  { id: "b3", name: "Tiny Axolotl SMP", host: "mc.tinyaxolotl.gg", port: 19134, favourite: false, fromServer: false },
];

let activeBackend: string | null = "b1";

function renderBackends(): void {
  const host = q("[data-proxy-list]");
  if (!host) return;
  const plates = BACKENDS.map<Plate>((b) => ({
    id: b.id,
    name: b.name,
    detail: `${b.host}:${b.port}`,
    glyphKey: b.host,
    chips: [
      ...(b.id === activeBackend && world.proxyUp
        ? ([["Forwarding here", "ok"]] as const)
        : ([] as const)),
      ...(b.fromServer ? ([["From your server", "muted"]] as const) : ([] as const)),
    ],
    favourite: b.favourite,
    active: b.id === activeBackend && world.proxyUp,
    reachable: world.cap !== "off",
    readonly: b.fromServer,
  }));
  const state = stateFor();
  host.innerHTML = listHtml(
    state,
    BACKENDS.length,
    plates.map(plateHtml).join("") + addTile("Add a server"),
    {
      failTitle:
        world.cap === "off"
          ? "This server will not accept a proxy"
          : "Couldn't load your servers",
      failNote:
        world.cap === "off"
          ? "Bedrock support is turned off on <b>bvc.alaydriem.com</b>, so position sent from a " +
            "proxy is discarded. Ask the operator to turn it on, or switch to a server that has it."
          : "The server did not answer when we asked for its list. Your own entries are still here " +
            "once it does.",
      retry: "Check again",
      emptyTitle: "No servers yet",
      emptyNote:
        "Add the address you would have joined in Minecraft, and the proxy will sit in front of it.",
      emptyAction: '<button class="rad-btn rad-btn--primary" data-plate-add>Add a server</button>',
    },
  );
  hydrate(host);
}

let proxyUptime = 2531;

function paintProxyStatus(): void {
  const chip = q("[data-proxy-chip]");
  const note = q("[data-proxy-note]");
  const stop = q<HTMLButtonElement>("[data-proxy-stop]");
  const backend = BACKENDS.find((b) => b.id === activeBackend);
  if (chip) {
    chip.textContent = world.proxyUp ? "Listening" : "Stopped";
    chip.className = `rad-status-chip rad-status-chip--${world.proxyUp ? "ok" : "muted"}`;
  }
  if (note) {
    note.textContent = world.proxyUp
      ? `Running for ${clock(proxyUptime)} · forwarding to ${backend?.host ?? "nothing"}`
      : "Pick where you play, then connect.";
  }
  if (stop) {
    stop.hidden = !world.proxyUp;
  }
}

/* ---------------------------------------------------------------- realms */

interface Realm {
  id: string;
  name: string;
  motd: string;
  open: boolean;
  favourite: boolean;
}

const REALMS: Realm[] = [
  { id: "r1", name: "Alaydriem's Realm", motd: "Survival, no resets", open: true, favourite: true },
  { id: "r2", name: "Hearthhold", motd: "Building server", open: true, favourite: false },
  { id: "r3", name: "Tiny Axolotl SMP", motd: "Back on Fridays", open: false, favourite: false },
];

let activeRealm: string | null = "r1";

function renderRealms(): void {
  const host = q("[data-realm-list]");
  if (!host) return;
  const plates = REALMS.map<Plate>((r) => ({
    id: r.id,
    name: r.name,
    detail: r.motd,
    glyphKey: `${r.name}-realm`,
    chips: [
      ...(r.id === activeRealm ? ([["Forwarding here", "ok"]] as const) : ([] as const)),
      ...(r.open ? ([] as const) : ([["Closed", "muted"]] as const)),
    ],
    favourite: r.favourite,
    active: r.id === activeRealm,
    reachable: r.open && world.cap !== "off",
    readonly: true,
  }));
  host.innerHTML = listHtml(stateFor(), REALMS.length, plates.map(plateHtml).join(""), {
    failTitle: world.cap === "off" ? "This server will not accept a Realm" : "Couldn't load your Realms",
    failNote:
      world.cap === "off"
        ? "Bedrock support is turned off on <b>bvc.alaydriem.com</b>, so a Realm has nowhere to " +
          "send position. Nothing here will have an effect until the operator turns it on."
        : "Xbox Live returned 503. Nothing is wrong with your account &mdash; it is usually over " +
          "within a few minutes.",
    retry: "Try again",
    emptyTitle: "No Realms on this account",
    emptyNote:
      "Realms you own or have been invited to appear here. If you have just been invited, accept " +
      "it in Minecraft first.",
  });
  hydrate(host);
  const backend = q("[data-realm-backend]");
  const chosen = REALMS.find((r) => r.id === activeRealm);
  if (backend) {
    backend.textContent = chosen?.name ?? "Nothing yet";
    backend.className = `rad-status-chip rad-status-chip--${chosen ? "ok" : "muted"}`;
  }
}

/* ---- plate wiring, shared ---- */

let editing: Backend | null = null;

frame.addEventListener("click", (e) => {
  const target = e.target as HTMLElement;

  const fav = target.closest<HTMLElement>("[data-plate-fav]");
  if (fav) {
    const id = fav.dataset.plateFav ?? "";
    const backend = BACKENDS.find((b) => b.id === id);
    const realm = REALMS.find((r) => r.id === id);
    if (backend) backend.favourite = !backend.favourite;
    if (realm) realm.favourite = !realm.favourite;
    renderBackends();
    renderRealms();
    return;
  }

  const go = target.closest<HTMLElement>("[data-plate-go]");
  if (go) {
    const id = go.dataset.plateGo ?? "";
    if (BACKENDS.some((b) => b.id === id)) {
      activeBackend = id;
      world.proxyUp = true;
      proxyUptime = 0;
      syncWorld();
      renderBackends();
      paintProxyStatus();
      Toast.show("Proxy started — join 127.0.0.1:19132");
    } else {
      activeRealm = id;
      renderRealms();
      Toast.show("Proxy pointed at the Realm — join 127.0.0.1:19132");
    }
    return;
  }

  const stop = target.closest<HTMLElement>("[data-plate-stop]");
  if (stop) {
    if (BACKENDS.some((b) => b.id === stop.dataset.plateStop)) {
      world.proxyUp = false;
      syncWorld();
      renderBackends();
      paintProxyStatus();
    } else {
      activeRealm = null;
      renderRealms();
    }
    return;
  }

  const plateMenu = target.closest<HTMLElement>("[data-plate-menu]");
  if (plateMenu) {
    const backend = BACKENDS.find((b) => b.id === plateMenu.dataset.plateMenu);
    if (!backend) return;
    const entries: MenuEntry[] = [{ label: "Edit…" }, MENU_DIVIDER, { label: "Remove", danger: true }];
    menu.open(plateMenu, entries, (item) => {
      if (item.label === "Remove") {
        BACKENDS = BACKENDS.filter((b) => b !== backend);
        if (activeBackend === backend.id) activeBackend = null;
        renderBackends();
        paintProxyStatus();
        return;
      }
      openBackend(backend);
    });
    return;
  }

  if (target.closest("[data-plate-add]")) openBackend(null);
  if (target.closest("[data-proxy-stop]")) {
    world.proxyUp = false;
    syncWorld();
    renderBackends();
    paintProxyStatus();
  }
  // A retry is only honest if it can succeed: the demo's failure is either the
  // capability or the fetch, and this clears whichever one is on.
  if (target.closest("[data-list-retry]")) {
    world.list = "loading";
    if (world.cap === "off") world.cap = "unknown";
    renderBackends();
    renderRealms();
    window.setTimeout(() => {
      world.list = "ready";
      world.cap = "on";
      for (const b of document.querySelectorAll<HTMLElement>("[data-g-cap]")) {
        b.textContent = CAP_LABEL.on;
        b.setAttribute("aria-pressed", "true");
      }
      for (const b of document.querySelectorAll<HTMLElement>("[data-g-list]")) {
        b.textContent = LIST_LABEL.ready;
        b.setAttribute("aria-pressed", "true");
      }
      syncWorld();
      renderBackends();
      renderRealms();
    }, 1200);
    return;
  }
  if (target.closest("[data-cap-retry]")) {
    world.cap = "on";
    for (const b of document.querySelectorAll<HTMLElement>("[data-g-cap]")) {
      b.textContent = CAP_LABEL.on;
      b.setAttribute("aria-pressed", "true");
    }
    syncWorld();
    renderBackends();
    Toast.show("This server supports Bedrock");
  }
  if (target.closest("[data-devicecode-done]")) {
    world.xbl = true;
    for (const b of document.querySelectorAll<HTMLElement>('[data-g-state="xbl"]')) {
      b.setAttribute("aria-pressed", "true");
    }
    syncWorld();
    modal.close();
    Toast.show("Signed in — your Realms are listed");
  }
});

function openBackend(backend: Backend | null): void {
  editing = backend;
  const title = q("[data-backend-title]");
  if (title) title.textContent = backend ? "Edit this server" : "Add a server";
  const name = q<HTMLInputElement>("[data-backend-name]");
  const host = q<HTMLInputElement>("[data-backend-host]");
  if (name) name.value = backend?.name ?? "";
  if (host) host.value = backend ? `${backend.host}:${backend.port}` : "";
  modal.open("backend");
  name?.focus();
}

q("[data-backend-go]")?.addEventListener("click", () => {
  const name = q<HTMLInputElement>("[data-backend-name]")?.value.trim() ?? "";
  const address = q<HTMLInputElement>("[data-backend-host]")?.value.trim() ?? "";
  if (!name || !address) {
    Toast.show("A name and an address, please");
    return;
  }
  const [host, port] = address.split(":");
  if (editing) {
    editing.name = name;
    editing.host = host;
    editing.port = Number(port) || 19132;
  } else {
    BACKENDS = [
      ...BACKENDS,
      { id: `b${BACKENDS.length + 1}`, name, host, port: Number(port) || 19132, favourite: false, fromServer: false },
    ];
  }
  editing = null;
  modal.close();
  renderBackends();
  paintProxyStatus();
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
  let lastLine = 0;
  AnimationLoop.shared().add((t) => {
    if (t - lastLine < 2200) return;
    lastLine = t;
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

/* ---------------------------------------------------------------- about */

let checking = false;

function paintUpdate(): void {
  const title = q("[data-upd-title]");
  const note = q("[data-upd-note]");
  const control = q("[data-upd-control]");
  if (!title || !note || !control) return;
  if (checking) {
    title.textContent = "Checking for an update";
    note.textContent = "Asking the update server.";
    control.innerHTML = '<span class="rad-status-chip rad-status-chip--live">Checking</span>';
    return;
  }
  if (world.update) {
    title.textContent = "Version 1.0.0-beta.9 is ready to install";
    note.textContent = "Installs on the next restart. Nothing is downloaded until you say so.";
    control.innerHTML =
      '<button class="rad-btn rad-btn--primary" data-upd-install><span data-rad-icon="download"></span> Install</button>';
  } else {
    title.textContent = "You are up to date";
    note.textContent = "Last checked just now.";
    control.innerHTML = '<button class="rad-btn" data-upd-check>Check again</button>';
  }
  IconBinding.sync(control);
}

frame.addEventListener("click", (e) => {
  const target = e.target as HTMLElement;
  if (target.closest("[data-upd-check]")) {
    checking = true;
    paintUpdate();
    window.setTimeout(() => {
      checking = false;
      paintUpdate();
      renderNav();
    }, 1100);
    return;
  }
  if (target.closest("[data-upd-install]")) {
    Toast.show("Downloading — the app restarts when it is ready");
    return;
  }
  if (target.closest("[data-export-logs]")) {
    Toast.show("Logs written to Documents / BVC");
    return;
  }
  if (target.closest("[data-flags-refresh]")) {
    const note = q("[data-flags-note]");
    if (note) note.textContent = "Checked just now. Nothing changed.";
    return;
  }
});

/**
 * The platform identifier, behind three presses on the release type.
 *
 * It is support's first question and nobody else's business: a row that is always
 * there invites a player to read a machine ID as something they are supposed to
 * understand. Three presses is the well-worn shape for exactly this.
 */
let variantPresses = 0;
let variantTimer = 0;
q("[data-variant]")?.addEventListener("click", () => {
  variantPresses++;
  window.clearTimeout(variantTimer);
  variantTimer = window.setTimeout(() => (variantPresses = 0), 1400);
  if (variantPresses < 3) return;
  variantPresses = 0;
  for (const el of frame.querySelectorAll<HTMLElement>("[data-platform-row]")) el.hidden = false;
  Toast.show("Platform identifier shown — support asks for this");
});

/* ---------------------------------------------------------------- boot */

renderNav();
goToPane("account");
setMobileLevel("list");
syncWorld();
recordings.render();
library.render();
renderBackends();
renderRealms();
renderPlayers();
paintProxyStatus();
paintUpdate();
syncAddress(true);
syncInterface();
renderClients(0);

// One second, shared: the proxy's uptime and every client's connected time are the
// same clock, and two intervals would let them disagree on screen.
let ticked = 0;
AnimationLoop.shared().add((t) => {
  if (t - ticked < 1000) return;
  ticked = t;
  if (world.proxyUp) proxyUptime++;
  paintProxyStatus();
  renderClients(Math.floor(t / 1000));
});
