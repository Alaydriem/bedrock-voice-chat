import { GlyphBinding } from "$radial/bindings/GlyphBinding";
import { IconBinding } from "$radial/bindings/IconBinding";
import { LevelMeterBinding } from "$radial/bindings/LevelMeterBinding";
import type { RingBinding } from "$radial/bindings/RingBinding";
import type { ScopeBinding } from "$radial/bindings/ScopeBinding";
import { AnimationLoop } from "$radial/core/canvas/AnimationLoop";
import { ChatLog } from "$radial/core/controllers/ChatLog";
import { Diagnostics, type DiagnosticsInput } from "$radial/core/controllers/Diagnostics";
import { Handoff, type Point } from "$radial/core/controllers/Handoff";
import { KvGridView } from "$radial/core/controllers/KvGridView";
import { MENU_DIVIDER, Menu } from "$radial/core/controllers/Menu";
import { GroupName } from "$radial/core/naming/GroupName";
import { SelfState } from "$radial/core/controllers/SelfState";
import { Sheet } from "$radial/core/controllers/Sheet";
import { SwipeActions } from "$radial/core/controllers/SwipeActions";
import { Toast } from "$radial/core/controllers/Toast";
import { PlayerHue } from "$radial/core/sources/PlayerHue";
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

/** Above this many in a section, a card each stops fitting and avatars take over. */
const DENSE_AT = 16;

/**
 * Cards a section keeps once it is dense. Everybody else is an avatar.
 *
 * The nearest six, not the six who are talking. Promoting on speech means the set
 * changes every couple of seconds, which reflows every avatar below it and needs a
 * demotion hold to be bearable — and an avatar that pulses already answers "who is
 * talking" without anything moving.
 */
const CARD_LIMIT = 6;

interface Player {
  name: string;
  /** `game:gamertag`, lowercased. The key the glyph, the hue and the roster agree on. */
  cn: string;
  hue: string;
  bearing: number;
  /** Metres. Meaningless for a channel member, who is full volume at any distance. */
  distance: number;
  /** True for a channel member: full volume at any distance. */
  inGroup: boolean;
  /** In the world with no BVC connection. Nothing said here reaches them. */
  offVoice: boolean;
  gain: number;
  muted: boolean;
  source: SyntheticLevelSource;
}

/**
 * `PlayerHue` lowercases its key so one player is one identity whatever the casing, and
 * `ServerGlyph` hashes what it is handed — so the glyph is keyed on the lowercased CN
 * too. Otherwise the blocks and the ring around them are two different colours for the
 * same person.
 */
function player(name: string, bearing: number, distance: number, inGroup: boolean, phase: number): Player {
  const cn = `minecraft:${name}`.toLowerCase();
  return {
    name,
    cn,
    hue: PlayerHue.of(cn),
    bearing,
    distance: inGroup ? 0 : distance,
    inGroup,
    offVoice: false,
    gain: 1,
    muted: false,
    source: new SyntheticLevelSource({ phase }),
  };
}

const NEARBY_NAMES = [
  "Alaydriem", "Petra", "Juno", "Kestrel", "Fennec", "Bram",
  "Sable", "Corvid", "Thistle", "Onyx", "Marrow", "Pike", "Quill", "Rook",
  "Slate", "Tamsin", "Umber", "Vesper", "Willa", "Yarrow", "Zephyr", "Ash",
  "Bracken", "Cinder", "Dray", "Elm", "Flint", "Gorse", "Hollow", "Iris",
  "Juniper", "Knot", "Larkspur", "Mica", "Nettle", "Oleander", "Pyre", "Quarry",
  "Reed", "Sorrel", "Teasel", "Ulmus", "Verbena", "Wold", "Xanthe", "Yew",
];

const GROUP_NAMES = ["Moth", "Wren", "Hollis", "Ivory"];

const ROSTER: Player[] = NEARBY_NAMES.map((name, i) =>
  // Bearing walks the circle by an irrational-ish step so no two neighbours land on the
  // same spoke, and distance ascends: nearest first is the only ordering worth trusting
  // once there are forty of them.
  player(name, (i * 2.39996) % (Math.PI * 2), 4 + i * 0.92, false, i * 0.83),
);

const GROUP: Player[] = GROUP_NAMES.map((name, i) =>
  player(name, (i * 1.7) % (Math.PI * 2), 0, true, 2.4 + i * 0.9),
);

const ALL = [...ROSTER, ...GROUP];

/** Scattered, so an off-voice player reads as a neighbour rather than a straggler. */
const OFF_VOICE_ORDER = [2, 9, 17, 26, 33, 44];
const OFF_VOICE_STEPS = [0, 1, 3, 6];

const SERVERS = [
  { host: "bvc.alaydriem.com", name: "Alaydriem's SMP" },
  { host: "voice.hearthhold.net", name: "Hearthhold" },
  { host: "bvc.tinyaxolotl.gg", name: "Tiny Axolotl" },
];

/**
 * `owned` is what gates renaming and closing, and it is a different question to `joined`.
 * Somebody coordinating several groups is in at most one of them at a time and still owns all
 * of them, so gating the admin actions on membership would lock them out of every group but the
 * one they are sitting in.
 *
 * `activeAt` is seconds ago rather than a timestamp: the gallery has no clock to be relative to.
 */
/**
 * The session actions, defined once and rendered on both surfaces.
 *
 * The sheet's rows in `dashboard.html` are this list; the desktop dropdown is built from it. Two
 * hand-written copies is how they drifted apart in the first place.
 */
const SESSION_ACTIONS: readonly { label: string; danger?: boolean }[] = [
  { label: "Add a server" },
  { label: "Settings" },
  { label: "Connection status" },
  { label: "Sign out", danger: true },
];

const GROUPS = [
  { id: "raid", name: "Raid party", members: 2, owned: true, activeAt: 40, roster: ["Moth", "Wren"] },
  {
    id: "build",
    name: "Ocean Monument Expedition",
    members: 4,
    owned: false,
    activeAt: 320,
    roster: ["Hollis", "Ivory", "Juno", "Petra"],
  },
  { id: "nether", name: "Nether run", members: 0, owned: true, activeAt: null, roster: [] },
];

const state = {
  nearby: 0,
  groupId: null as string | null,
  offVoiceStep: 0,
  /** The one avatar opened for adjustment, expanded where it stands. */
  opened: null as string | null,
  server: 0,
  showingRoster: false,
  statusOpen: false,
  mobileTab: "roster",
  /** Seen by the feed but not close enough to hear. Drives the ring's middle register. */
  approaching: 0,
  link: "up" as "up" | "reconnecting" | "down",
  /** The group whose action tray is out, and the one whose editor is open. */
  trayId: null as string | null,
  editId: null as string | null,
};

const self = new SelfState();
const mySource = new SyntheticLevelSource({ phase: 0.9 });

const near = () => ROSTER.slice(0, state.nearby);
const groupSize = () => GROUPS.find((g) => g.id === state.groupId)?.members ?? 0;
const grouped = () => (state.groupId ? GROUP.slice(0, groupSize()) : []);
const visible = () => [...near(), ...grouped()];
const groupName = () => GROUPS.find((g) => g.id === state.groupId)?.name ?? "";

function applyOffVoice(): void {
  const chosen = new Set(OFF_VOICE_ORDER.slice(0, OFF_VOICE_STEPS[state.offVoiceStep]));
  ROSTER.forEach((p, i) => (p.offVoice = chosen.has(i)));
}

/* ---------------------------------------------------------------- the ring */

const idleRing = mounted.byId.get("idle-ring") as RingBinding | undefined;
const idle = q("[data-idle]");
const roster = q("[data-roster]");
const headline = q("[data-headline]");

/** Scope the approach marks fall off over, in metres. Three times the voice range. */
const FEED_SCOPE = 240;

/**
 * The ring's three registers, which is the whole reason the feed reaches past voice range.
 *
 * Scanning says the system is listening — `live` with nothing placed on it, the register the
 * loader shows while it waits. With somebody approaching it reaches out to them: `lock` for one,
 * because one source acquiring is exactly what `lock` documents, and `live` for several. `empty`
 * is left for the one case that really is nothing happening, a link that is down — drawing marks
 * there would assert positions this client can no longer be told about.
 *
 * Mirrors `RingCast.of` in the app, which is the same rule against real positions.
 */
AnimationLoop.shared().add((t) => {
  if (!idleRing || state.showingRoster) return;

  if (state.link === "down") {
    idleRing.mode = "empty";
    idleRing.setSources([]);
    return;
  }

  if (state.approaching === 0) {
    idleRing.mode = "live";
    idleRing.setSources([]);
    return;
  }

  const sources = ROSTER.slice(0, Math.min(state.approaching, 5))
    .map((p) =>
      // Volume from distance rather than from their voice: they are out of earshot, so there is
      // no voice to draw. What the mark carries is how close they are.
      PositionalSource.toRingSource({ bearing: p.bearing, distance: p.distance, hue: p.hue }, 1, FEED_SCOPE),
    )
    .filter((source): source is RingSource => source !== null);

  idleRing.mode = state.approaching === 1 ? "lock" : "live";
  idleRing.setSources(sources);
});

const idleKicker = q("[data-idle-kicker]");
const idleHeadline = q("[data-idle-headline]");
const idleNote = q("[data-idle-note]");
const idleStatus = q("[data-idle-status]");

/**
 * The words under the ring.
 *
 * The object does not change between these states, only what is said about it — which is why a
 * link that is down is the resting ring plus a sentence rather than a different component.
 */
function paintIdle(): void {
  const down = state.link === "down";
  const retrying = state.link === "reconnecting";
  const n = state.approaching;

  idleKicker.textContent = down ? "Disconnected" : retrying ? "Reconnecting" : n ? "Approaching" : "Listening";

  idleHeadline.textContent = down
    ? "Not connected"
    : retrying
      ? "Trying to reach the server"
      : n === 0
        ? "Nobody nearby"
        : n === 1
          ? `Somebody ${Math.round(ROSTER[0].distance)} m away`
          : `${n} people nearby`;

  idleNote.textContent = down || retrying
    ? "Nobody can hear you, and you cannot hear anyone. This clears itself when the link comes back."
    : n === 0
      ? "Voices appear here the moment someone walks into range. Nothing to join, nothing to dial."
      : "Close enough to see, not close enough to hear. They arrive on the list when they are.";

  // The refresh action is gone, so this is the route from noticing to diagnosing.
  idleStatus.hidden = !(down || retrying);
}

/* ---------------------------------------------------------------- the roster */

const meters = new Map<string, LevelMeterBinding>();
const chips = new Map<string, HTMLElement>();

/**
 * The whole roster: the group's section, then earshot's.
 *
 * The group leads, always. Joining a channel is deliberate and proximity is ambient, so
 * the people you went out of your way to talk to are never under a list of neighbours.
 *
 * @param arriving Whether a flyer is coming for these cards, which is the only reason to
 *   hold one back. A re-render triggered by a tap has no flyer, and a card left pending
 *   with nothing to land on it never becomes visible again.
 */
function renderRoster(arriving: boolean): void {
  for (const meter of meters.values()) meter.destroy();
  meters.clear();

  const leave = '<button class="rad-leave-btn" data-leave>Leave</button>';
  const group = grouped();

  // A channel you are alone in still needs its rule and its way out. Rendering nothing
  // because the member list is empty leaves you joined with no visible exit — and being
  // first into a group you just made is the normal way to arrive in one.
  const groupHtml = state.groupId
    ? group.length
      ? section(groupName(), group, arriving, leave)
      : `<div class="rad-roster__section">${sectionRule(groupName(), 0, leave)}` +
        '<p class="rad-roster__empty">Nobody else is in here yet.</p></div>'
    : "";

  roster.innerHTML = groupHtml + section("In earshot", near(), arriving);

  for (const canvas of roster.querySelectorAll<HTMLCanvasElement>("canvas[data-rad-level]")) {
    const p = ALL.find((x) => x.name === canvas.dataset.player);
    if (!p) continue;
    meters.set(
      p.name,
      new LevelMeterBinding(canvas, {
        source: p.muted ? undefined : p.source,
        color: p.hue,
        cell: 3,
        onLive: (live) => canvas.closest(".rad-player")?.classList.toggle("is-live", live),
      }),
    );
  }

  chips.clear();
  for (const el of roster.querySelectorAll<HTMLElement>("[data-chip]")) {
    chips.set(el.dataset.chip ?? "", el);
  }

  mountGlyphs(roster);
  IconBinding.sync(roster);
}

/**
 * An avatar's ring, written as a custom property rather than drawn.
 *
 * Forty avatars would be forty canvases on the animation loop, all painting the same
 * thing at different amplitudes. One style write each, fifteen times a second — roughly
 * the rate real level events arrive at — costs nothing and looks the same.
 */
const CHIP_LEVEL_INTERVAL_MS = 66;

let chipLevelAt = 0;

AnimationLoop.shared().add((t) => {
  if (t - chipLevelAt < CHIP_LEVEL_INTERVAL_MS) return;
  chipLevelAt = t;

  const deafened = self.snapshot.deafened;
  for (const [name, el] of chips) {
    const p = ALL.find((x) => x.name === name);
    if (!p) continue;
    const live = !p.muted && !p.offVoice && !deafened;
    el.style.setProperty("--level", live ? p.source.level.toFixed(3) : "0");
  }
});

function sectionRule(title: string, count: number, extra = ""): string {
  return (
    `<div class="rad-section-rule"><span class="rad-section-rule__title">${title}</span>` +
    `<span class="rad-section-rule__line"></span>${extra}` +
    `<span class="rad-section-rule__count">${count}</span></div>`
  );
}

function section(title: string, list: Player[], arriving: boolean, extra = ""): string {
  if (!list.length) return "";

  const cards = list.length < DENSE_AT ? list : list.slice(0, CARD_LIMIT);
  const rest = list.slice(cards.length);

  let html =
    sectionRule(title, list.length, extra) +
    `<div class="rad-card-grid">${cards.map((p) => card(p, false, arriving)).join("")}</div>`;

  if (rest.length) {
    html +=
      '<div class="rad-avatar-grid">' +
      rest
        .map((p) =>
          state.opened === p.name
            ? chip(p) + `<div class="rad-avatar-grid__open">${card(p, true, false)}</div>`
            : chip(p),
        )
        .join("") +
      "</div>";
  }

  return `<div class="rad-roster__section">${html}</div>`;
}

/**
 * A player's face, in the same language a server's is.
 *
 * `ServerGlyph` derives a mirrored block pattern and a hue from any name, so a player
 * with no gamerpic still has an identity that every client agrees on. Real art overrides
 * it; the derived glyph is the floor, because it is the one guaranteed to exist.
 */
function identity(p: Player, positional: string, size: number): string {
  return (
    `<span class="rad-server-id ${positional}">` +
    `<canvas data-rad-glyph="${p.cn}" data-size="${size}"></canvas></span>`
  );
}

function mountGlyphs(root: ParentNode): void {
  for (const canvas of root.querySelectorAll<HTMLCanvasElement>(
    "canvas[data-rad-glyph]:not([data-mounted])",
  )) {
    canvas.dataset.mounted = "true";
    new GlyphBinding(canvas, canvas.dataset.radGlyph ?? "", {
      size: Number(canvas.dataset.size ?? 36),
    });
  }
}

/**
 * The full card. An off-voice player gets no gain controls rather than disabled ones:
 * there is no audio to turn down, and a greyed slider reads as "muted", which is a
 * different and wrong story.
 */
function card(p: Player, dismissable: boolean, arriving: boolean): string {
  const range = p.inGroup ? "IN GROUP" : `${Math.round(p.distance)} M`;

  const body = p.offVoice
    ? '<div class="rad-player__note"><span data-rad-icon="unlink"></span>' +
      "In range, not on voice &mdash; they will not hear you</div>"
    : `<div class="rad-player__level"><canvas data-rad-level data-player="${p.name}"></canvas></div>` +
      '<div class="rad-player__controls">' +
      `<button class="rad-player__mute" data-mute="${p.name}" aria-pressed="${p.muted}" ` +
      `aria-label="${p.muted ? "Unmute" : "Mute"} ${p.name}"><span data-rad-icon="${p.muted ? "micoff" : "mic"}"></span></button>` +
      `<input class="rad-range" type="range" min="0" max="1.5" step="0.05" value="${p.gain}" ` +
      `data-gain="${p.name}"${p.muted ? " disabled" : ""} aria-label="Volume for ${p.name}" />` +
      `<span class="rad-player__percent">${Math.round(p.gain * 100)}%</span>` +
      "</div>";

  return (
    `<div class="rad-player${arriving ? " is-pending" : ""}` +
    `${p.offVoice ? " rad-player--game" : ""}" ` +
    `data-card="${p.name}" style="color:${p.hue}">` +
    '<div class="rad-player__avatar">' +
    `${identity(p, "rad-player__id", 54)}<span class="rad-player__ring"></span></div>` +
    '<div><div class="rad-player__head">' +
    `<span class="rad-player__name">${p.name}</span>` +
    `<span class="rad-player__distance">${range}</span>` +
    (dismissable
      ? '<button class="rad-player__dismiss" data-dismiss aria-label="Close this card">' +
        '<span data-rad-icon="close"></span></button>'
      : "") +
    `</div>${body}</div></div>`
  );
}

function chip(p: Player): string {
  const badge = p.offVoice
    ? '<span class="rad-avatar-chip__badge"><span data-rad-icon="unlink"></span></span>'
    : "";
  const open = state.opened === p.name;
  return (
    `<button class="rad-avatar-chip${p.offVoice ? " rad-avatar-chip--game" : ""}` +
    `${open ? " is-pinned" : ""}" ` +
    `data-chip="${p.name}" style="color:${p.hue}" aria-expanded="${open}" ` +
    `aria-label="Adjust ${p.name}">` +
    '<span class="rad-avatar-chip__face">' +
    identity(p, "rad-avatar-chip__id", 46) +
    `<span class="rad-avatar-chip__ring"></span>${badge}</span>` +
    `<span class="rad-avatar-chip__name">${p.name}</span>` +
    `<span class="rad-avatar-chip__meta">${p.inGroup ? "GROUP" : `${Math.round(p.distance)} M`}</span>` +
    "</button>"
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
        const p = ALL.find((x) => x.name === cardEl.dataset.card);
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
  renderRoster(true);
  roster.classList.remove("is-gone");
  idle.classList.add("is-gone");

  const cards = [...roster.querySelectorAll<HTMLElement>(".rad-player")];
  if (!was) {
    for (const cardEl of cards) {
      const p = ALL.find((x) => x.name === cardEl.dataset.card);
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
  const target = e.target as HTMLElement;

  const mute = target.closest<HTMLElement>("[data-mute]");
  if (mute) {
    const p = ALL.find((x) => x.name === mute.dataset.mute);
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

  if (target.closest("[data-dismiss]")) {
    state.opened = null;
    renderRoster(false);
    return;
  }

  if (target.closest("[data-leave]")) {
    joinGroup(null);
    return;
  }

  // An avatar opens where it stands. Promoting it to the top of the roster put the card
  // off-screen for anyone scrolled into a long list, so the tap appeared to do nothing.
  const chipEl = target.closest<HTMLElement>("[data-chip]");
  if (chipEl) {
    const name = chipEl.dataset.chip ?? null;
    state.opened = state.opened === name ? null : name;
    renderRoster(false);
  }
});

roster.addEventListener("input", (e) => {
  const slider = (e.target as HTMLElement).closest<HTMLInputElement>("[data-gain]");
  if (!slider) return;
  const p = ALL.find((x) => x.name === slider.dataset.gain);
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

/** "active 2 min ago", or nothing when no join or leave has been seen. */
function since(secondsAgo: number | null): string {
  if (secondsAgo === null) return "";
  if (secondsAgo < 60) return "active just now";
  const minutes = Math.round(secondsAgo / 60);
  if (minutes < 60) return `active ${minutes} min ago`;
  return `active ${Math.round(minutes / 60)} h ago`;
}

/** Beyond this the cluster is a smear; the count carries the rest. */
const CLUSTER_SHOWN = 4;

/**
 * A group row, and the actions behind it.
 *
 * No level meter. The server routes a channel's audio to its members alone, so a client outside
 * one receives nothing to measure and a meter here would be an invention — it used to draw one,
 * which is the thing this row is most wrong about. What can be said honestly is who is in it,
 * which of them can currently be heard, and when somebody last came or went.
 *
 * The tray sits under the row rather than beside it, so revealing it costs no layout, and it is
 * only rendered when there is something in it: a row you neither own nor belong to has no
 * actions, and a swipe that uncovers nothing teaches that the gesture does not work here.
 * Leaving follows membership; renaming and closing follow ownership.
 */
function groupRow(g: (typeof GROUPS)[number]): string {
  const joined = state.groupId === g.id;
  const open = state.trayId === g.id;
  const count = g.members + (joined ? 1 : 0);
  const stamp = since(g.activeAt);

  const shown = g.roster.slice(0, CLUSTER_SHOWN);
  const extra = Math.max(0, g.roster.length - CLUSTER_SHOWN);
  const cluster =
    `<span class="rad-group-cluster">` +
    shown
      .map(
        (name) =>
          // Dimmed for whoever cannot be heard from here, which is everyone in a group you are
          // not in. The CN form, so the glyph matches the one on that player's card.
          `<span class="rad-group-cluster__face ${joined ? "" : "is-quiet"}" title="${name}">` +
          `<span class="rad-server-id rad-group-cluster__id">` +
          `<canvas data-rad-glyph="minecraft:${name.toLowerCase()}" data-size="22"></canvas>` +
          `</span></span>`,
      )
      .join("") +
    (extra ? `<span class="rad-group-cluster__more">+${extra}</span>` : "") +
    `</span>`;

  const actions =
    (joined
      ? `<button class="rad-swipe__action" data-group-leave="${g.id}"><span data-rad-icon="unlink"></span> Leave</button>`
      : "") +
    (g.owned
      ? `<button class="rad-swipe__action" data-group-edit="${g.id}"><span data-rad-icon="gear"></span> Edit</button>` +
        `<button class="rad-swipe__action rad-swipe__action--danger" data-group-close="${g.id}">` +
        `<span data-rad-icon="trash"></span> Close</button>`
      : "");

  const row =
    `<div class="rad-swipe__track" data-group-track="${g.id}">` +
    `<button class="rad-group-row ${joined ? "is-on" : ""}" data-group="${g.id}">` +
    `<span class="rad-group-row__text"><span class="rad-group-row__name">${g.name}</span>` +
    (stamp ? `<span class="rad-group-row__since">${stamp}</span>` : "") +
    `</span>${cluster}<span class="rad-group-row__count">${count}</span>` +
    `</button></div>`;

  const swipe =
    `<div class="rad-swipe ${open && actions ? "is-open" : ""}">` +
    (actions ? `<div class="rad-swipe__tray" data-group-tray="${g.id}">${actions}</div>` : "") +
    row +
    `</div>`;

  if (state.editId !== g.id) return swipe;

  // The id is here rather than behind a details view because it is the reason an owner opens this
  // panel at all: the mod is addressed by id, and an id you cannot copy is one you transcribe off
  // a screen.
  return (
    swipe +
    `<div class="rad-group-edit">` +
    `<label class="rad-group-edit__row"><span class="rad-label">Group name</span>` +
    `<span class="rad-input"><input value="${g.name}" data-group-name aria-label="Group name" /></span></label>` +
    `<button class="rad-group-edit__id" data-group-copy="${g.id}">` +
    `<span class="rad-label">Group id</span><code>${g.id}</code>` +
    `<span class="rad-group-edit__copy"><span data-rad-icon="copy"></span> Copy</span></button>` +
    `<div class="rad-group-edit__foot">` +
    `<button class="rad-btn rad-btn--quiet" data-group-cancel>Cancel</button>` +
    `<button class="rad-btn" data-group-save="${g.id}">Save</button>` +
    `</div></div>`
  );
}

function renderGroups(): void {
  // The new-group row lives inside the card, so the list is one surface rather than a button
  // above a stack of separately-rounded ones.
  const html =
    `<button class="rad-new-group" data-group-new><span data-rad-icon="plus"></span> New group</button>` +
    GROUPS.map(groupRow).join("");

  for (const host of [q("[data-groups]"), q("[data-sheet-groups]")]) {
    host.innerHTML = html;
    mountGlyphs(host);
    IconBinding.sync(host);
  }
}

/* ------------------------------------------------- group actions, on a swipe */

let drag: { id: string; startX: number; wasOpen: boolean; swiped: boolean } | null = null;

function trackOf(id: string): HTMLElement | null {
  return frame.querySelector<HTMLElement>(`[data-group-track="${id}"]`);
}

function trayWidth(id: string): number {
  return frame.querySelector<HTMLElement>(`[data-group-tray="${id}"]`)?.clientWidth ?? 0;
}

function settle(id: string, open: boolean): void {
  const track = trackOf(id);
  if (!track) return;
  track.classList.remove("is-dragging");
  track.style.transform = `translateX(${SwipeActions.resting(open, trayWidth(id))}px)`;
}

frame.addEventListener("pointerdown", (e) => {
  const track = (e.target as HTMLElement).closest<HTMLElement>("[data-group-track]");
  const id = track?.dataset.groupTrack;
  if (!track || !id || !trayWidth(id) || state.editId === id) return;
  drag = { id, startX: (e as PointerEvent).clientX, wasOpen: state.trayId === id, swiped: false };
});

frame.addEventListener("pointermove", (e) => {
  if (!drag) return;
  const dx = (e as PointerEvent).clientX - drag.startX;
  if (!drag.swiped && !SwipeActions.isSwipe(dx)) return;
  drag.swiped = true;
  const track = trackOf(drag.id);
  if (!track) return;
  track.classList.add("is-dragging");
  track.style.transform = `translateX(${SwipeActions.offset(dx, trayWidth(drag.id), drag.wasOpen)}px)`;
});

window.addEventListener("pointerup", (e) => {
  if (!drag) return;
  const held = drag;
  drag = null;
  if (!held.swiped) return;

  const dx = (e as PointerEvent).clientX - held.startX;
  const offset = SwipeActions.offset(dx, trayWidth(held.id), held.wasOpen);
  const latched = SwipeActions.latches(offset, trayWidth(held.id));
  state.trayId = latched ? held.id : null;
  renderGroups();
  // Rendered first, so the element being settled is the one now in the document.
  if (latched) settle(held.id, true);
});

function joinGroup(id: string | null): void {
  state.groupId = id;
  prunePin();
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
  const target = e.target as HTMLElement;

  if (target.closest("[data-group-new]")) {
    // Create, join, and open for editing. Creating a group you are not in is never what was
    // meant, and the generated name is a suggestion the editor opens on.
    const id = `g${GROUPS.length + 1}`;
    const name = GroupName.next(GROUPS.map((g) => g.name));
    GROUPS.push({ id, name, members: 0, owned: true, activeAt: 0, roster: [] });
    state.editId = id;
    joinGroup(id);
    Toast.show("Group created");
    return;
  }

  const leave = target.closest<HTMLElement>("[data-group-leave]");
  if (leave) {
    state.trayId = null;
    joinGroup(null);
    Toast.show("Left the group");
    return;
  }

  const edit = target.closest<HTMLElement>("[data-group-edit]");
  if (edit) {
    state.trayId = null;
    state.editId = edit.dataset.groupEdit ?? null;
    renderGroups();
    return;
  }

  const closed = target.closest<HTMLElement>("[data-group-close]");
  if (closed) {
    const id = closed.dataset.groupClose;
    const at = GROUPS.findIndex((g) => g.id === id);
    if (at >= 0) GROUPS.splice(at, 1);
    state.trayId = null;
    state.editId = null;
    if (state.groupId === id) joinGroup(null);
    else renderGroups();
    Toast.show("Group closed");
    return;
  }

  const copy = target.closest<HTMLElement>("[data-group-copy]");
  if (copy) {
    void navigator.clipboard?.writeText(copy.dataset.groupCopy ?? "").catch(() => {});
    Toast.show("Group id copied");
    return;
  }

  const save = target.closest<HTMLElement>("[data-group-save]");
  if (save) {
    const field = frame.querySelector<HTMLInputElement>("[data-group-name]");
    const g = GROUPS.find((x) => x.id === save.dataset.groupSave);
    const name = field?.value.trim();
    if (g && name) g.name = name;
    state.editId = null;
    renderGroups();
    paintSelf();
    return;
  }

  if (target.closest("[data-group-cancel]")) {
    state.editId = null;
    renderGroups();
    return;
  }

  const group = target.closest<HTMLElement>("[data-group]");
  if (group) {
    // A tap that was really the end of a swipe takes no action: a gesture that reveals
    // actions must not also pick one.
    if (state.trayId === group.dataset.group) {
      state.trayId = null;
      renderGroups();
      return;
    }
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

  /**
   * The chevron, the corner glyph and the slide-up sheet reach one set of actions.
   *
   * These were three separately-written lists: the chevron offered "Switch server" and no
   * status, the sheet offered status and no identity. Nobody can learn a menu that changes
   * depending on which edge of the screen they came in from.
   *
   * A phone gets the sheet whichever one was pressed — a dropdown anchored to a control at the
   * bottom of the screen flips upward and lands under the thumb that opened it.
   */
  const identity = target.closest<HTMLElement>(".rad-self__id");
  if (identity) {
    if (frame.clientWidth <= 560) {
      sheet.open("servers", identity);
      return;
    }
    menu.open(
      identity,
      [
        { label: "Alaydriem", hint: "signed in" },
        MENU_DIVIDER,
        ...SERVERS.map((s, i) => ({
          label: s.host,
          hint: i === state.server ? "current" : `as Alaydriem`,
          on: i === state.server,
        })),
        MENU_DIVIDER,
        ...SESSION_ACTIONS.filter((a) => !a.danger).map((a) => ({ label: a.label })),
        MENU_DIVIDER,
        ...SESSION_ACTIONS.filter((a) => a.danger).map((a) => ({ label: a.label, danger: true })),
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
  if (!text || chatTarget.kind === "unavailable") return;
  chat.add({ author: "Alaydriem", text, fromApp: true });
  chatInput.value = "";
  chatSend.classList.remove("is-ready");
  if (!chat.isOpen) setChat(true);
}

/* ---------------------------------------------------------------- chat target

   One BVC server can host several worlds, and a message has to land in exactly one of
   them. Three cases, and only the last one has any UI at all:

     in game      the server already knows where you are standing. No choice is offered,
                  because the only thing a choice can do here is put your message in front
                  of the wrong people.
     one world    nothing to pick. Same words, no affordance.
     several      you are not in game and have been in more than one. Now it is a button.

   `unavailable` is orthogonal to all three: a world can be up and hosting while the chat
   channel behind it is down, and the composer has to say so rather than going quiet.
*/

type ChatTarget =
  | { kind: "in-game"; world: string }
  | { kind: "only"; world: string }
  | { kind: "choose"; world: string }
  | { kind: "unavailable"; reason: string };

const WORLDS = [
  { name: "Survival", seen: "in game now", available: true },
  { name: "Creative Flats", seen: "last seen 2 h ago", available: true },
  { name: "Skyblock", seen: "last seen yesterday", available: false },
];

const chatTargetBtn = q("[data-chat-target]");
const chatTargetName = q("[data-chat-target-name]");
const chatStatus = q("[data-chat-status]");
const chatBar = q(".rad-chat-bar");
const chatWorldRows = q("[data-sheet-chat-worlds]");
const chatNotice = q("[data-chat-notice]");

let chatTarget: ChatTarget = { kind: "in-game", world: "Survival" };

function applyChatTarget(): void {
  chatTargetBtn.classList.toggle("is-static", chatTarget.kind !== "choose");

  if (chatTarget.kind === "unavailable") {
    chatTargetName.textContent = "Server chat";
    chatStatus.className = "rad-status-chip rad-status-chip--warn";
    chatStatus.textContent = "Off";
    chatBar.classList.add("is-unavailable");
    chatInput.disabled = true;
    chatInput.value = "";
    chatInput.placeholder = chatTarget.reason;
    chatSend.classList.remove("is-ready");
    renderChatWorlds();
    return;
  }

  chatTargetName.textContent = chatTarget.world;
  chatStatus.className = "rad-status-chip rad-status-chip--live";
  chatStatus.textContent = chatTarget.kind === "in-game" ? "In game" : "Live";
  chatBar.classList.remove("is-unavailable");
  chatInput.disabled = false;
  // The target is named where the typing happens, not only in the header. The scrollback
  // is shut most of the time, and the moment that matters is the one where somebody is
  // deciding what to send.
  chatInput.placeholder = `Message ${chatTarget.world}…`;
  renderChatWorlds();
}

function renderChatWorlds(): void {
  const current = "world" in chatTarget ? chatTarget.world : "";
  chatWorldRows.innerHTML = WORLDS.map((w, i) => {
    const on = w.name === current;
    return (
      `<button class="rad-sheet-row ${on ? "is-on" : ""}" data-chat-world="${w.name}"` +
      ` style="--i:${i}"${w.available ? "" : " disabled"}>` +
      '<span class="rad-sheet-row__text">' +
      `<span class="rad-sheet-row__name">${w.name}</span>` +
      `<span class="rad-sheet-row__host">${w.available ? w.seen : "chat unavailable"}</span>` +
      "</span>" +
      (on ? '<span class="rad-sheet-row__tick" data-rad-icon="check"></span>' : "") +
      "</button>"
    );
  }).join("");
  IconBinding.sync(chatWorldRows);

  for (const row of chatWorldRows.querySelectorAll<HTMLElement>("[data-chat-world]")) {
    row.addEventListener("click", () => {
      chatTarget = { kind: "choose", world: row.dataset.chatWorld ?? "" };
      applyChatTarget();
      sheet.close();
    });
  }
}

applyChatTarget();

// Driven from the gallery bar, not a timer. Somebody comparing "in game" against "several"
// needs both on demand; a rotation makes them wait for the state they wanted and makes the
// difference between the two hard to see at all.
const TARGET_STATES: Record<string, ChatTarget> = {
  "in-game": { kind: "in-game", world: "Survival" },
  only: { kind: "only", world: "Survival" },
  choose: { kind: "choose", world: "Creative Flats" },
  unavailable: { kind: "unavailable", reason: "Survival is not running chat right now" },
  // Not a target state of its own: a send the server refused because the player moved worlds
  // while the app still held the old one. Shown here so the kit's notice has a specimen.
  moved: { kind: "only", world: "Creative Flats" },
};

for (const button of document.querySelectorAll<HTMLElement>('[data-act="chat-target"]')) {
  button.addEventListener("click", () => {
    const key = button.dataset.target ?? "in-game";
    chatTarget = TARGET_STATES[key] ?? TARGET_STATES["in-game"];
    chatNotice.hidden = key !== "moved";
    for (const b of document.querySelectorAll<HTMLElement>('[data-act="chat-target"]')) {
      b.setAttribute("aria-pressed", String(b === button));
    }
    applyChatTarget();
    // The head lives inside the scrollback, so opening it is the only way the switch shows
    // anything at all.
    if (!chat.isOpen) setChat(true);
  });
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
    capturing: 50,
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
    // On in this demo, so the row shows both of the states that matter as you talk and
    // stop. A gate that is switched off reads "off" instead, which is the third.
    noiseGate: s.transmitting ? "Open" : "Closed",
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

const maxNearby = ROSTER.length;

function setFaultLabel(): void {
  const button = document.querySelector<HTMLElement>('[data-act="fault"]');
  if (button) button.textContent = `Fault: ${FAULTS[link.fault]}`;
}

/** An opened avatar who has left earshot would leave a card nobody can dismiss. */
function prunePin(): void {
  if (state.opened && !visible().some((p) => p.name === state.opened)) state.opened = null;
}

function syncControls(): void {
  const plus = document.querySelector<HTMLButtonElement>('[data-act="plus"]');
  const minus = document.querySelector<HTMLButtonElement>('[data-act="minus"]');
  if (plus) plus.disabled = state.nearby >= maxNearby;
  if (minus) minus.disabled = state.nearby <= 0;
  const ptt = document.querySelector<HTMLElement>('[data-act="ptt"]');
  if (ptt) ptt.setAttribute("aria-pressed", String(self.snapshot.mode === "ptt"));

  const game = document.querySelector<HTMLElement>('[data-act="game"]');
  if (game) {
    const count = OFF_VOICE_STEPS[state.offVoiceStep];
    game.textContent = `Off voice: ${count}`;
    game.setAttribute("aria-pressed", String(count > 0));
  }

  for (const b of document.querySelectorAll<HTMLElement>('[data-act="preset"]')) {
    b.setAttribute("aria-pressed", String(Number(b.dataset.n) === state.nearby));
  }

  const approach = document.querySelector<HTMLElement>('[data-act="approach"]');
  if (approach) {
    approach.textContent = `Approaching: ${state.approaching}`;
    approach.setAttribute("aria-pressed", String(state.approaching > 0));
  }

  const link = document.querySelector<HTMLElement>('[data-act="link"]');
  if (link) {
    link.textContent = `Link: ${state.link}`;
    link.setAttribute("aria-pressed", String(state.link !== "up"));
  }
}

for (const button of document.querySelectorAll<HTMLElement>("[data-act]")) {
  button.addEventListener("click", () => {
    switch (button.dataset.act) {
      case "plus":
        if (state.nearby < maxNearby) state.nearby++;
        prunePin();
        render();
        break;
      case "minus":
        if (state.nearby > 0) state.nearby--;
        prunePin();
        render();
        break;
      case "preset":
        state.nearby = Math.min(maxNearby, Number(button.dataset.n ?? 0));
        prunePin();
        render();
        break;
      case "game":
        state.offVoiceStep = (state.offVoiceStep + 1) % OFF_VOICE_STEPS.length;
        applyOffVoice();
        render();
        break;
      case "ptt":
        self.setMode(self.snapshot.mode === "ptt" ? "activated" : "ptt");
        break;
      case "status":
        setStatus(!state.statusOpen);
        break;
      case "approach":
        // 0 -> 1 -> 3 -> 5, which walks scanning, lock and live.
        state.approaching = state.approaching === 0 ? 1 : state.approaching === 1 ? 3 : state.approaching === 3 ? 5 : 0;
        paintIdle();
        syncControls();
        break;
      case "link":
        state.link = state.link === "up" ? "reconnecting" : state.link === "reconnecting" ? "down" : "up";
        // A card that survives a dead link asserts that person can hear you, which is the
        // misleading kind of wrong — it keeps somebody talking into nothing.
        if (state.link !== "up") {
          state.nearby = 0;
          state.groupId = null;
          state.opened = null;
          renderGroups();
          render();
          paintSelf();
        }
        paintIdle();
        syncControls();
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
        state.offVoiceStep = 0;
        state.opened = null;
        state.approaching = 0;
        state.link = "up";
        state.trayId = null;
        state.editId = null;
        paintIdle();
        applyOffVoice();
        link.fault = 0;
        link.drops = 0;
        setFaultLabel();
        for (const p of ALL) {
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

applyOffVoice();
renderRail();
renderGroups();
paintIdle();
buildSelf();
paintSelf();
render();
paintStatus();
syncControls();
