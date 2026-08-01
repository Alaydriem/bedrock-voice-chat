import { GlyphBinding } from "$radial/bindings/GlyphBinding";
import { IconBinding } from "$radial/bindings/IconBinding";
import { LevelMeterBinding } from "$radial/bindings/LevelMeterBinding";
import { MarkBinding } from "$radial/bindings/MarkBinding";
import type { RingBinding } from "$radial/bindings/RingBinding";
import type { ScopeBinding } from "$radial/bindings/ScopeBinding";
import { AnimationLoop } from "$radial/core/canvas/AnimationLoop";
import { ChatLog } from "$radial/core/controllers/ChatLog";
import { Diagnostics, type DiagnosticsInput } from "$radial/core/controllers/Diagnostics";
import { Handoff, type Point } from "$radial/core/controllers/Handoff";
import { KvGridView } from "$radial/core/controllers/KvGridView";
import { MENU_DIVIDER, Menu } from "$radial/core/controllers/Menu";
import { SelfState } from "$radial/core/controllers/SelfState";
import { Sheet } from "$radial/core/controllers/Sheet";
import { Toast } from "$radial/core/controllers/Toast";
import { MarkData } from "$radial/core/mark/MarkData";
import { PositionalSource } from "$radial/core/sources/PositionalSource";
import { SyntheticLevelSource } from "$radial/core/sources/SyntheticLevelSource";
import type { RingSource } from "$radial/core/ring/RingSource";
import { Gallery } from "./gallery";

const mounted = Gallery.boot();

const frame = Gallery.requireElement("dsh-frame");

const q = <T extends HTMLElement>(sel: string): T => {
  const el = frame.querySelector<T>(sel);
  if (!el) throw new Error(`dashboard: missing ${sel}`);
  return el;
};

/* ---------------------------------------------------------------- the world */

interface Player {
  name: string;
  hue: string;
  bearing: number;
  /** Metres. */
  distance: number;
  /** True for a channel member: full volume at any distance. */
  inGroup: boolean;
  gain: number;
  muted: boolean;
  source: SyntheticLevelSource;
}

function player(name: string, column: number, bearing: number, distance: number, inGroup: boolean, phase: number): Player {
  return {
    name,
    hue: MarkData.hueAt(column),
    bearing,
    distance: distance * PositionalSource.RANGE,
    inGroup,
    gain: 1,
    muted: false,
    source: new SyntheticLevelSource({ phase }),
  };
}

const ROSTER: Player[] = [
  player("Alaydriem", 7, 0.55, 0.3, false, 0),
  player("Petra", 12, 2.4, 0.52, false, 1.7),
  player("Juno", 18, 4.3, 0.74, false, 3.4),
  player("Kestrel", 3, 5.55, 0.44, false, 5.1),
  player("Moth", 10, 1.55, 0.62, true, 2.4),
  player("Wren", 15, 3.3, 0.86, true, 4.2),
];

const SERVERS = [
  { host: "bvc.alaydriem.com", name: "Alaydriem's SMP" },
  { host: "voice.hearthhold.net", name: "Hearthhold" },
  { host: "bvc.tinyaxolotl.gg", name: "Tiny Axolotl" },
];

const GROUPS = [
  { id: "raid", name: "Raid party", members: 2, phase: 0.3 },
  { id: "build", name: "Ocean Monument Expedition", members: 4, phase: 2.1 },
  { id: "nether", name: "Nether run", members: 0, phase: 4.4 },
];

const state = {
  nearby: 0,
  groupId: null as string | null,
  server: 0,
  showingRoster: false,
  statusOpen: false,
  mobileTab: "roster",
};

const self = new SelfState();
const mySource = new SyntheticLevelSource({ phase: 0.9 });

const near = () => ROSTER.filter((p) => !p.inGroup).slice(0, state.nearby);
const grouped = () => (state.groupId ? ROSTER.filter((p) => p.inGroup) : []);
const visible = () => [...near(), ...grouped()];
const groupName = () => GROUPS.find((g) => g.id === state.groupId)?.name ?? "";

/* ---------------------------------------------------------------- the ring */

const idleRing = mounted.byId.get("idle-ring") as RingBinding | undefined;
const idle = q("[data-idle]");
const roster = q("[data-roster]");
const headline = q("[data-headline]");

AnimationLoop.shared().add((t) => {
  if (!idleRing || state.showingRoster) return;
  // Nobody in earshot, so nothing is placed on it: the at-rest ring says the system is
  // listening rather than pretending there is something to hear.
  idleRing.mode = "empty";
  idleRing.setSources([]);
});

/* ---------------------------------------------------------------- the roster */

const meters = new Map<string, LevelMeterBinding>();

function renderRoster(): void {
  for (const meter of meters.values()) meter.destroy();
  meters.clear();

  const sections: string[] = [];
  const section = (title: string, list: Player[], extra = "") =>
    list.length
      ? `<div class="rad-section-rule"><span class="rad-section-rule__title">${title}</span>` +
        `<span class="rad-section-rule__line"></span>${extra}` +
        `<span class="rad-section-rule__count">${list.length}</span></div>` +
        `<div class="rad-card-grid">${list.map(card).join("")}</div>`
      : "";

  sections.push(section("In earshot", near()));
  sections.push(
    section(
      groupName(),
      grouped(),
      state.groupId ? '<button class="rad-leave-btn" data-leave>Leave</button>' : "",
    ),
  );
  roster.innerHTML = sections.join("");

  for (const canvas of roster.querySelectorAll<HTMLCanvasElement>("canvas[data-rad-level]")) {
    const p = ROSTER.find((x) => x.name === canvas.dataset.player);
    if (!p) continue;
    meters.set(
      p.name,
      new LevelMeterBinding(canvas, {
        source: p.source,
        color: p.hue,
        cell: 3,
        onLive: (live) => canvas.closest(".rad-player")?.classList.toggle("is-live", live),
      }),
    );
  }
  IconBinding.sync(roster);
}

function card(p: Player): string {
  const range = p.inGroup ? "IN GROUP" : `${Math.round(p.distance)} M`;
  return (
    `<div class="rad-player is-pending" data-card="${p.name}" style="color:${p.hue}">` +
    `<div class="rad-player__avatar" style="background:${p.hue}">` +
    `<span class="rad-player__ring"></span>${p.name.slice(0, 2).toUpperCase()}</div>` +
    '<div><div class="rad-player__head">' +
    `<span class="rad-player__name">${p.name}</span>` +
    `<span class="rad-player__distance">${range}</span></div>` +
    `<div class="rad-player__level"><canvas data-rad-level data-player="${p.name}"></canvas></div>` +
    '<div class="rad-player__controls">' +
    `<button class="rad-player__mute" data-mute="${p.name}" aria-pressed="${p.muted}" ` +
    `aria-label="${p.muted ? "Unmute" : "Mute"} ${p.name}"><span data-rad-icon="${p.muted ? "micoff" : "mic"}"></span></button>` +
    `<input class="rad-range" type="range" min="0" max="1.5" step="0.05" value="${p.gain}" ` +
    `data-gain="${p.name}"${p.muted ? " disabled" : ""} aria-label="Volume for ${p.name}" />` +
    `<span class="rad-player__percent">${Math.round(p.gain * 100)}%</span>` +
    "</div></div></div>"
  );
}

/**
 * Where a player's bar sits on the ring, in viewport coordinates. The flyer leaves from
 * there rather than from the centre, so the animation is telling the truth about which
 * bar was theirs.
 */
function ringPoint(p: Player): Point {
  const geometry = idleRing?.geometry;
  // Before the ring's first frame there is no geometry to read, so the flyer starts
  // from the middle of the empty state rather than from nowhere.
  if (!idleRing || !geometry) return Handoff.centreOf(idle);
  return Handoff.ringPoint(idleRing.canvas, geometry, p.bearing);
}

function render(): void {
  const was = state.showingRoster;
  const list = visible();
  state.showingRoster = list.length > 0;

  if (!state.showingRoster) {
    const leaving = [...roster.querySelectorAll<HTMLElement>(".rad-player")];
    idle.classList.remove("is-gone");
    if (was) {
      for (const cardEl of leaving) {
        const p = ROSTER.find((x) => x.name === cardEl.dataset.card);
        const avatar = cardEl.querySelector(".rad-player__avatar");
        if (!p || !avatar) continue;
        void Handoff.fly(Handoff.centreOf(avatar), ringPoint(p), p.hue);
      }
    }
    roster.classList.add("is-gone");
    setTimeout(() => {
      if (!state.showingRoster) roster.innerHTML = "";
    }, 340);
    headline.textContent = "NOBODY IN EARSHOT";
    return;
  }

  headline.textContent = `${list.length} PLAYER${list.length === 1 ? "" : "S"}`;
  renderRoster();
  roster.classList.remove("is-gone");
  idle.classList.add("is-gone");

  const cards = [...roster.querySelectorAll<HTMLElement>(".rad-player")];
  if (!was) {
    for (const cardEl of cards) {
      const p = ROSTER.find((x) => x.name === cardEl.dataset.card);
      const avatar = cardEl.querySelector(".rad-player__avatar");
      if (!p || !avatar) continue;
      void Handoff.fly(ringPoint(p), Handoff.centreOf(avatar), p.hue).then(() =>
        cardEl.classList.remove("is-pending"),
      );
    }
    // Backstop: reduced motion resolves fly() immediately, and a card must never be
    // left invisible because an animation did not happen.
    setTimeout(() => {
      for (const c of cards) c.classList.remove("is-pending");
    }, 620);
  } else {
    cards.forEach((c, i) => setTimeout(() => c.classList.remove("is-pending"), 40 + i * 50));
  }
}

/* ---------------------------------------------------------------- roster interaction */

roster.addEventListener("click", (e) => {
  const mute = (e.target as HTMLElement).closest<HTMLElement>("[data-mute]");
  if (mute) {
    const p = ROSTER.find((x) => x.name === mute.dataset.mute);
    if (!p) return;
    p.muted = !p.muted;
    mute.setAttribute("aria-pressed", String(p.muted));
    const icon = mute.querySelector<HTMLElement>("[data-rad-icon]");
    if (icon) new IconBinding(icon, p.muted ? "micoff" : "mic");
    const slider = mute.closest(".rad-player__controls")?.querySelector<HTMLInputElement>("[data-gain]");
    if (slider) slider.disabled = p.muted;
    meters.get(p.name)?.setSource(p.muted ? null : p.source);
    return;
  }
  if ((e.target as HTMLElement).closest("[data-leave]")) joinGroup(null);
});

roster.addEventListener("input", (e) => {
  const slider = (e.target as HTMLElement).closest<HTMLInputElement>("[data-gain]");
  if (!slider) return;
  const p = ROSTER.find((x) => x.name === slider.dataset.gain);
  if (!p) return;
  p.gain = Number(slider.value);
  const percent = slider.closest(".rad-player__controls")?.querySelector(".rad-player__percent");
  if (percent) percent.textContent = `${Math.round(p.gain * 100)}%`;
});

/* ---------------------------------------------------------------- rail, groups, sheets */

const sheet = new Sheet(frame);
const menu = new Menu(q("[data-rad-menu]"), frame);

function renderRail(): void {
  const html = SERVERS.map(
    (s, i) =>
      `<button class="rad-rail-item ${i === state.server ? "is-on" : ""}" data-server="${i}" title="${s.name}">` +
      `<canvas data-rad-glyph="${s.host}" data-size="36"></canvas></button>`,
  ).join("");
  q("[data-rail]").innerHTML = html;

  q("[data-sheet-servers]").innerHTML = SERVERS.map(
    (s, i) =>
      `<button class="rad-sheet-row ${i === state.server ? "is-on" : ""}" data-server="${i}">` +
      `<canvas data-rad-glyph="${s.host}" data-size="30"></canvas>` +
      `<span class="rad-sheet-row__text"><span class="rad-sheet-row__name">${s.name}</span>` +
      `<span class="rad-sheet-row__host">${s.host}</span></span>` +
      (i === state.server ? '<span class="rad-sheet-row__tick" data-rad-icon="check"></span>' : "") +
      "</button>",
  ).join("");

  for (const canvas of frame.querySelectorAll<HTMLCanvasElement>("canvas[data-rad-glyph]:not([data-mounted])")) {
    canvas.dataset.mounted = "true";
    new GlyphBinding(canvas, canvas.dataset.radGlyph ?? "", { size: Number(canvas.dataset.size ?? 36) });
  }
  IconBinding.sync(frame);
}

const groupMeters = new Map<string, MarkBinding>();

function renderGroups(): void {
  const html = GROUPS.map((g) => {
    const on = state.groupId === g.id;
    return (
      `<button class="rad-group-row ${on ? "is-on" : ""}" data-group="${g.id}">` +
      `<span class="rad-group-row__name">${g.name}</span>` +
      `<canvas data-group-level="${g.id}"></canvas>` +
      `<span class="rad-group-row__count">${g.members + (on ? 1 : 0)}</span></button>`
    );
  }).join("");
  q("[data-groups]").innerHTML = html;
  q("[data-sheet-groups]").innerHTML = html;

  for (const meter of groupMeters.values()) meter.destroy();
  groupMeters.clear();
  for (const canvas of frame.querySelectorAll<HTMLCanvasElement>("canvas[data-group-level]")) {
    const g = GROUPS.find((x) => x.id === canvas.dataset.groupLevel);
    if (!g) continue;
    // A group's own level: whether anything is happening in there, without joining.
    const source = new SyntheticLevelSource({ phase: g.phase, peak: g.members ? 0.9 : 0 });
    groupMeters.set(
      `${g.id}-${canvas.dataset.groupLevel}`,
      new MarkBinding(canvas, { cell: 2, mortar: false, color: state.groupId === g.id ? "#bb8dfa" : "#8a76b4" }),
    );
    const binding = groupMeters.get(`${g.id}-${canvas.dataset.groupLevel}`);
    if (binding) source.subscribe((level) => (binding.gain = Math.max(0.05, level)));
  }
}

function joinGroup(id: string | null): void {
  state.groupId = id;
  renderGroups();
  render();
  paintSelf();
  q("[data-groups-btn]").classList.toggle("has-badge", Boolean(id));
}

frame.addEventListener("click", (e) => {
  const server = (e.target as HTMLElement).closest<HTMLElement>("[data-server]");
  if (server) {
    state.server = Number(server.dataset.server);
    state.nearby = 0;
    state.groupId = null;
    q("[data-server-name]").textContent = SERVERS[state.server].name;
    renderRail();
    renderGroups();
    render();
    sheet.close();
    syncControls();
    return;
  }
  const group = (e.target as HTMLElement).closest<HTMLElement>("[data-group]");
  if (group) {
    joinGroup(state.groupId === group.dataset.group ? null : (group.dataset.group ?? null));
    sheet.close();
  }
});

/* ---------------------------------------------------------------- self controls */

const pillHost = q("[data-self-pill]");
const capsuleHost = q("[data-self-capsule]");

/**
 * The self controls, built once.
 *
 * Built once and then mutated in place, never re-rendered. Replacing `innerHTML` on a
 * state change destroys the element and creates a new one already in the end state, and
 * a CSS transition cannot run on an element that never existed in the start state — so
 * every animation on these buttons silently became a jump. The record button's grow was
 * the visible symptom; the same rebuild also threw away and remounted two canvases and
 * their level bindings on every mute press.
 */
function selfMarkup(capsule: boolean): string {
  return (
    `<div class="${capsule ? "rad-self-capsule" : "rad-self-pill"}">` +
    '<span class="rad-self__avatar">AL</span>' +
    '<button class="rad-self__id" title="Profile and sign-out">' +
    '<span class="rad-self__name"><span>Alaydriem</span><span class="rad-health-dot"></span>' +
    '<span class="rad-self__chev">&#9660;</span></span>' +
    `<span class="rad-self__sub"><canvas data-my-meter data-cell="${capsule ? 2 : 3}"></canvas>` +
    '<span class="rad-self__state" data-self-state></span></span></button>' +
    `<button class="rad-self__btn ${capsule ? "rad-self__btn--primary" : ""}" ` +
    'data-self-mute aria-pressed="false" aria-label="Mute">' +
    '<span data-rad-icon="mic"></span></button>' +
    '<button class="rad-self__btn rad-self__btn--deafen" data-self-deafen ' +
    'aria-pressed="false" aria-label="Deafen">' +
    '<span data-rad-icon="head"></span></button>' +
    (capsule
      ? ""
      : '<button class="rad-self__btn rad-self__btn--record" data-self-record ' +
        'aria-pressed="false" aria-label="Start recording">' +
        '<span data-rad-icon="rec"></span>' +
        '<span class="rad-self__rec-time" data-rec-time>00:00</span></button>') +
    "</div>"
  );
}

const myMeters: LevelMeterBinding[] = [];

function buildSelf(): void {
  pillHost.innerHTML = selfMarkup(false);
  capsuleHost.innerHTML = selfMarkup(true);
  for (const canvas of frame.querySelectorAll<HTMLCanvasElement>("canvas[data-my-meter]")) {
    myMeters.push(
      new LevelMeterBinding(canvas, {
        source: mySource,
        color: "rainbow",
        cell: Number(canvas.dataset.cell ?? 3),
      }),
    );
  }
  IconBinding.sync(frame);
}

function paintSelf(): void {
  const s = self.snapshot;
  const ptt = s.mode === "ptt";

  frame.classList.toggle("is-muted", s.muted && !s.deafened);
  frame.classList.toggle("is-deafened", s.deafened);
  roster.classList.toggle("is-deafened", s.deafened);

  for (const button of frame.querySelectorAll<HTMLElement>("[data-self-mute]")) {
    button.setAttribute("aria-pressed", String(s.muted));
    button.classList.toggle("is-holding", ptt && s.holding);
    button.setAttribute("aria-label", ptt ? "Hold to talk" : s.muted ? "Unmute" : "Mute");
    const icon = button.querySelector<HTMLElement>("[data-rad-icon]");
    if (icon) new IconBinding(icon, s.muted && !ptt ? "micoff" : "mic");
  }

  for (const button of frame.querySelectorAll<HTMLElement>("[data-self-deafen]")) {
    button.setAttribute("aria-pressed", String(s.deafened));
    button.setAttribute("aria-label", s.deafened ? "Undeafen" : "Deafen");
    const icon = button.querySelector<HTMLElement>("[data-rad-icon]");
    if (icon) new IconBinding(icon, s.deafened ? "headoff" : "head");
  }

  for (const button of frame.querySelectorAll<HTMLElement>("[data-self-record]")) {
    button.setAttribute("aria-pressed", String(s.recording));
    button.setAttribute("aria-label", s.recording ? "Stop recording" : "Start recording");
  }

  for (const el of frame.querySelectorAll<HTMLElement>("[data-self-state]")) {
    el.textContent = groupName();
  }

  // Your own mark flatlines when you are not transmitting. One axis, one meaning —
  // and the source is swapped on the existing binding rather than remounting it.
  for (const meter of myMeters) {
    meter.setSource(s.transmitting ? mySource : null);
    meter.color = s.transmitting ? "rainbow" : "#7a68a0";
  }
}

self.subscribe(() => paintSelf());

frame.addEventListener("click", (e) => {
  const target = e.target as HTMLElement;
  const mute = target.closest<HTMLElement>("[data-self-mute]");
  if (mute) {
    if (self.snapshot.mode === "ptt") return;
    // The centre is read before the toggle, because the toggle re-renders the pill
    // and the button this handler was given stops existing.
    const from = Handoff.centreOf(mute);
    self.toggleMute();
    Handoff.burstAt(from, self.snapshot.muted ? "#ff8266" : "#5ce383");
    return;
  }
  const deafen = target.closest<HTMLElement>("[data-self-deafen]");
  if (deafen) {
    const from = Handoff.centreOf(deafen);
    self.toggleDeafen();
    Handoff.burstAt(from, self.snapshot.deafened ? "#ffcf4d" : "#5ce383");
    return;
  }
  if (target.closest("[data-self-record]")) self.toggleRecording(performance.now());

  // The chevron beside your name is a disclosure: it opens the account menu. It was
  // an affordance with nothing behind it, which is worse than no affordance.
  const identity = target.closest<HTMLElement>(".rad-self__id");
  if (identity) {
    menu.open(
      identity,
      [
        { label: "Alaydriem", hint: "signed in" },
        MENU_DIVIDER,
        { label: "Audio settings" },
        { label: "Linked accounts" },
        { label: "Switch server" },
        MENU_DIVIDER,
        { label: "Sign out of this server", danger: true },
      ],
      (item) => Toast.show(item.label),
    );
  }
});

frame.addEventListener("pointerdown", (e) => {
  if ((e.target as HTMLElement).closest("[data-self-mute]")) self.hold(true);
});
window.addEventListener("pointerup", () => self.hold(false));

AnimationLoop.shared().add((t) => {
  if (!self.snapshot.recording) return;
  const seconds = Math.floor(self.elapsed(t) / 1000);
  const text = `${String(Math.floor(seconds / 60)).padStart(2, "0")}:${String(seconds % 60).padStart(2, "0")}`;
  for (const el of frame.querySelectorAll<HTMLElement>("[data-rec-time]")) el.textContent = text;
});

/* ---------------------------------------------------------------- chat */

const chatBody = q("[data-chat-body]");
const chatBadge = q("[data-chat-badge]");
const chatDot = q("[data-chat-dot]");
const chatInput = q<HTMLInputElement>("[data-chat-input]");
const chatSend = q("[data-chat-send]");

const chat = new ChatLog({
  body: chatBody,
  hueOf: (author) => ROSTER.find((p) => p.name === author)?.hue ?? "#9483b6",
  onUnread: (count) => {
    chatBadge.hidden = count === 0;
    chatBadge.textContent = count > 9 ? "9+" : String(count);
    chatDot.hidden = count === 0;
  },
});

function setChat(open: boolean): void {
  frame.classList.toggle("is-chat", open);
  chat.setOpen(open);
}

q("[data-chat-toggle]").addEventListener("click", () => setChat(!chat.isOpen));
q("[data-chat-close]").addEventListener("click", () => setChat(false));
chatInput.addEventListener("focus", () => {
  if (!frame.classList.contains("rad-frame--phone")) setChat(true);
});
chatInput.addEventListener("input", () => {
  chatSend.classList.toggle("is-ready", chatInput.value.trim().length > 0);
});
chatInput.addEventListener("keydown", (e) => {
  if (e.key === "Enter") send();
});
chatSend.addEventListener("click", send);

function send(): void {
  const text = chatInput.value.trim();
  if (!text) return;
  chat.add({ author: "Alaydriem", text, fromApp: true });
  chatInput.value = "";
  chatSend.classList.remove("is-ready");
  if (!chat.isOpen) setChat(true);
}

const SCRIPT = [
  { author: "Petra", text: "anyone got spare iron? im 3 short for a hopper" },
  { system: true, text: "Kestrel joined the game" },
  { author: "Kestrel", text: "brb loading in, phone only for now", fromApp: true },
  { author: "Juno", text: "theres a ravine under the birch forest, full of exposed lapis" },
  { author: "Petra", text: "Alaydriem can you open the gate", mention: true },
  { system: true, text: "Moth was slain by an Enderman" },
  { author: "Moth", text: "i regret everything" },
  { author: "Juno", text: "coords 214 / 63 / -880 if anyone wants to come mine" },
  { system: true, text: "Wren left the game" },
];

for (const m of SCRIPT.slice(0, 3)) chat.add(m);

let scriptIndex = 3;
let chatAt = 0;
AnimationLoop.shared().add((t) => {
  if (t - chatAt < 6200) return;
  chatAt = t;
  chat.add(SCRIPT[scriptIndex % SCRIPT.length]);
  scriptIndex++;
});

for (const tab of frame.querySelectorAll<HTMLElement>("[data-tab]")) {
  tab.addEventListener("click", () => {
    state.mobileTab = tab.dataset.tab ?? "roster";
    for (const t of frame.querySelectorAll<HTMLElement>("[data-tab]")) {
      t.classList.toggle("is-on", t === tab);
    }
    setChat(state.mobileTab === "chat");
  });
}

/* ---------------------------------------------------------------- status */

const scope = mounted.byId.get("scope") as ScopeBinding | undefined;
const kv = new KvGridView(q("[data-kv]"));
const verdictEl = q("[data-verdict]");

const link = {
  rtt: 41,
  loss: 0.2,
  jitter: 42,
  drops: 0,
  uptime: 2531,
  inRate: 48000,
  port: 443,
  fault: 0,
  reconnecting: false,
};

const FAULTS = ["none", "others-muted", "loss", "rate", "reconnecting"] as const;

function diagnostics(): DiagnosticsInput {
  const s = self.snapshot;
  const mutedOthers = visible().filter((p) => p.muted).length;
  return {
    rtt: link.rtt,
    lossPercent: link.loss,
    jitterMs: link.jitter,
    jitterDrops: link.drops,
    datagramsIn: s.deafened ? 0 : Math.round(46 + Math.random() * 5),
    datagramsOut: s.transmitting ? Math.round(48 + Math.random() * 4) : 0,
    inputDevice: "Focusrite Scarlett 2i2",
    inputRate: link.inRate,
    outputDevice: "Sennheiser HD 560S",
    outputRate: 48000,
    quicPort: link.port,
    protocol: "1.3.0",
    rangeMetres: PositionalSource.RANGE,
    falloff: "inverse-square",
    server: SERVERS[state.server].host,
    uptimeSeconds: link.uptime,
    reconnecting: link.reconnecting || FAULTS[link.fault] === "reconnecting",
    attempt: 3,
    muted: s.muted,
    deafened: s.deafened,
    pttIdle: s.mode === "ptt" && !s.holding,
    mutedOthers,
    visiblePlayers: visible().length,
  };
}

function paintStatus(): void {
  const input = diagnostics();
  const [severity, text] = Diagnostics.verdict(input);
  const className = `rad-verdict${severity === "ok" ? "" : ` rad-verdict--${severity}`}`;
  if (verdictEl.className !== className) verdictEl.className = className;
  const textEl = verdictEl.querySelector(".rad-verdict__text");
  if (textEl && textEl.textContent !== text) textEl.textContent = text;
  kv.update(Diagnostics.groups(input));
}

let diagAt = 0;
AnimationLoop.shared().add((t) => {
  if (t - diagAt < 1000) return;
  diagAt = t;
  link.uptime++;
  const lossy = FAULTS[link.fault] === "loss";
  const spike = lossy ? 16 + Math.random() * 62 : Math.random() < 0.05 ? 16 : 0;
  const target = 34 + Math.sin(t * 0.00035) * 6 + Math.random() * 4 + spike;
  link.rtt = Math.max(9, Math.round(link.rtt * 0.45 + target * 0.55));
  link.loss = lossy ? Number((3.4 + Math.random() * 2.6).toFixed(1)) : Number((Math.random() * 0.5).toFixed(1));
  link.jitter = Math.round(38 + Math.random() * 10 + (lossy ? 45 : 0));
  if (lossy && Math.random() < 0.5) link.drops++;
  link.inRate = FAULTS[link.fault] === "rate" ? 44100 : 48000;
  scope?.push(link.rtt);
  if (state.statusOpen) paintStatus();
});

function setStatus(open: boolean): void {
  state.statusOpen = open;
  frame.classList.toggle("is-status", open);
  for (const b of frame.querySelectorAll<HTMLElement>("[data-peek]")) {
    b.classList.toggle("is-on", open);
    b.setAttribute("aria-pressed", String(open));
  }
  if (open) paintStatus();
}

for (const b of frame.querySelectorAll<HTMLElement>("[data-peek]")) {
  b.addEventListener("click", () => setStatus(!state.statusOpen));
}
q("[data-status-close]").addEventListener("click", () => setStatus(false));

// With the pull gesture gone, the sheet is the phone's route into status. It sits
// beside Settings and Refresh, which is where someone looking for it would go.
q("[data-sheet-status]").addEventListener("click", () => {
  sheet.close();
  setStatus(true);
});

const reconnect = q("[data-reconnect]");
reconnect.addEventListener("click", () => {
  if (link.reconnecting) return;
  link.reconnecting = true;
  reconnect.classList.add("is-busy");
  paintStatus();
  setTimeout(() => {
    link.reconnecting = false;
    reconnect.classList.remove("is-busy");
    link.uptime = 0;
    link.drops = 0;
    link.fault = 0;
    setFaultLabel();
    for (const p of ROSTER) p.muted = false;
    scope?.reset(36);
    render();
    paintStatus();
  }, 1700);
});

/* ---------------------------------------------------------------- demo controls */

const maxNearby = ROSTER.filter((p) => !p.inGroup).length;

function setFaultLabel(): void {
  const button = document.querySelector<HTMLElement>('[data-act="fault"]');
  if (button) button.textContent = `Fault: ${FAULTS[link.fault]}`;
}

function syncControls(): void {
  const plus = document.querySelector<HTMLButtonElement>('[data-act="plus"]');
  const minus = document.querySelector<HTMLButtonElement>('[data-act="minus"]');
  if (plus) plus.disabled = state.nearby >= maxNearby;
  if (minus) minus.disabled = state.nearby <= 0;
  const ptt = document.querySelector<HTMLElement>('[data-act="ptt"]');
  if (ptt) ptt.setAttribute("aria-pressed", String(self.snapshot.mode === "ptt"));
}

for (const button of document.querySelectorAll<HTMLElement>("[data-act]")) {
  button.addEventListener("click", () => {
    switch (button.dataset.act) {
      case "plus":
        if (state.nearby < maxNearby) state.nearby++;
        render();
        break;
      case "minus":
        if (state.nearby > 0) state.nearby--;
        render();
        break;
      case "ptt":
        self.setMode(self.snapshot.mode === "ptt" ? "activated" : "ptt");
        break;
      case "status":
        setStatus(!state.statusOpen);
        break;
      case "fault":
        link.fault = (link.fault + 1) % FAULTS.length;
        if (FAULTS[link.fault] === "others-muted") {
          ROSTER[0].muted = true;
          ROSTER[1].muted = true;
          render();
        } else {
          for (const p of ROSTER) p.muted = false;
        }
        link.drops = 0;
        setFaultLabel();
        paintStatus();
        break;
      case "reset":
        state.nearby = 0;
        state.groupId = null;
        link.fault = 0;
        link.drops = 0;
        setFaultLabel();
        for (const p of ROSTER) {
          p.gain = 1;
          p.muted = false;
        }
        self.reset();
        renderGroups();
        render();
        break;
    }
    syncControls();
  });
}

/* ---------------------------------------------------------------- boot */

renderRail();
renderGroups();
buildSelf();
paintSelf();
render();
paintStatus();
syncControls();
