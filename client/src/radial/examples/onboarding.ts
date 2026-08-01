import { IconBinding } from "$radial/bindings/IconBinding";
import { LevelMeterBinding } from "$radial/bindings/LevelMeterBinding";
import type { RingBinding } from "$radial/bindings/RingBinding";
import { TimelineBinding } from "$radial/bindings/TimelineBinding";
import { AnimationLoop } from "$radial/core/canvas/AnimationLoop";
import { ScreenRouter } from "$radial/core/controllers/ScreenRouter";
import { StepFlow } from "$radial/core/controllers/StepFlow";
import { Toast } from "$radial/core/controllers/Toast";
import { MarkData } from "$radial/core/mark/MarkData";
import { PositionalSource } from "$radial/core/sources/PositionalSource";
import { SyntheticLevelSource } from "$radial/core/sources/SyntheticLevelSource";
import type { RingSource } from "$radial/core/ring/RingSource";
import { Gallery } from "./gallery";

const mounted = Gallery.boot();

const frame = Gallery.requireElement("ob-frame");

/* ---------------------------------------------------------------- the cast
 * Hues come from columns of the mark, so a player's colour is provably part of the
 * palette rather than picked next to it.
 */
interface Player {
  name: string;
  hue: string;
  bearing: number;
  drift: number;
  /** Distance oscillation, so people walk toward you and away again. */
  radius: number;
  breathe: number;
  phase: number;
  source: SyntheticLevelSource;
}

const CAST: Player[] = [
  { name: "ALAYDRIEM", hue: MarkData.hueAt(7), bearing: 0.55, drift: 0.0004, radius: 0.3, breathe: 0.00009, phase: 0, source: new SyntheticLevelSource({ phase: 0 }) },
  { name: "PETRA", hue: MarkData.hueAt(12), bearing: 2.55, drift: -0.00029, radius: 0.54, breathe: 0.00006, phase: 1.7, source: new SyntheticLevelSource({ phase: 1.7 }) },
  { name: "JUNO", hue: MarkData.hueAt(18), bearing: 4.45, drift: 0.0002, radius: 0.82, breathe: 0.00004, phase: 3.1, source: new SyntheticLevelSource({ phase: 3.1 }) },
];

/** Where a player is at a moment: their angle, and how far out. */
function placement(p: Player, t: number) {
  const distance = Math.max(0.05, Math.min(1.14, p.radius + Math.sin(t * p.breathe + p.phase) * 0.13));
  return { bearing: p.bearing + t * p.drift, distance: distance * PositionalSource.RANGE, hue: p.hue };
}

/* ---------------------------------------------------------------- step 1 · the field */

const scene = mounted.byId.get("scene") as RingBinding | undefined;
const earshot = frame.querySelector<HTMLElement>("[data-earshot]");

const readoutHost = frame.querySelector<HTMLElement>("[data-readouts]");
const readouts = new Map<string, { row: HTMLElement; gain: HTMLElement; range: HTMLElement }>();
if (readoutHost) {
  readoutHost.innerHTML = CAST.map(
    (p, i) =>
      `<div class="rad-readout" data-readout="${p.name}">` +
      `<span class="rad-readout__dot" style="background:${p.hue}"></span>` +
      `<span>${p.name}</span>` +
      `<canvas data-rad-level data-color="${p.hue}" data-phase="${p.phase}" data-cell="3"></canvas>` +
      '<span class="rad-readout__gain">&mdash;</span>' +
      '<span class="rad-readout__range">&mdash;</span></div>',
  ).join("");
  for (const row of readoutHost.querySelectorAll<HTMLElement>("[data-readout]")) {
    readouts.set(row.dataset.readout ?? "", {
      row,
      gain: row.querySelector<HTMLElement>(".rad-readout__gain")!,
      range: row.querySelector<HTMLElement>(".rad-readout__range")!,
    });
  }
  // Mounted by hand rather than through Mount.scan, because these were created after
  // the initial scan ran.
  for (const canvas of readoutHost.querySelectorAll<HTMLCanvasElement>("canvas[data-rad-level]")) {
    new LevelMeterBinding(canvas, {
      source: new SyntheticLevelSource({ phase: Number(canvas.dataset.phase ?? 0) }),
      color: canvas.dataset.color,
      cell: 3,
    });
  }
}

let lastReadout = 0;
AnimationLoop.shared().add((t) => {
  const sources: RingSource[] = [];
  let audible = 0;

  for (const p of CAST) {
    const place = placement(p, t);
    const level = p.source.at(t);
    const source = PositionalSource.toRingSource(place, level);
    if (source) {
      sources.push(source);
      audible++;
    }

    // Readouts are text, so they update ten times a second rather than sixty: past
    // that the numbers are a blur and it costs layout for nothing.
    if (t - lastReadout < 100) continue;
    const readout = readouts.get(p.name);
    if (!readout) continue;
    const loud = source !== null;
    readout.row.classList.toggle("is-live", loud);
    readout.gain.textContent = loud ? `${Math.round(source.volume * 100)}%` : "––";
    readout.range.textContent = `${Math.round(place.distance)} m`;
  }
  if (t - lastReadout >= 100) lastReadout = t;

  scene?.setSources(sources);
  if (earshot) earshot.textContent = `80 M · ${audible} IN EARSHOT`;
});

/* ---------------------------------------------------------------- step 2 · channels
 * Membership moves on its own, because the thing being explained is that proximity
 * keeps running underneath a channel rather than being replaced by it.
 */

const assignment = [1, 1, 0];

function renderChannels(): void {
  for (let channel = 0; channel < 2; channel++) {
    const body = frame.querySelector<HTMLElement>(`[data-channel-body="${channel}"]`);
    if (!body) continue;
    const members = CAST.filter((_, i) => assignment[i] === channel);
    body.innerHTML = members.length
      ? members
          .map(
            (p) =>
              '<div class="rad-channel__member">' +
              `<canvas data-rad-level data-color="${p.hue}" data-phase="${p.phase}" data-cell="3"></canvas>` +
              `<span class="rad-channel__member-name">${p.name}</span></div>`,
          )
          .join("")
      : '<div class="rad-channel__empty">— nobody here —</div>';

    for (const canvas of body.querySelectorAll<HTMLCanvasElement>("canvas[data-rad-level]")) {
      new LevelMeterBinding(canvas, {
        // In a channel everyone is at full volume at any distance, so the meter is not
        // subject to falloff — which is the entire point of a channel.
        source: new SyntheticLevelSource({ phase: Number(canvas.dataset.phase ?? 0), peak: channel === 1 ? 1 : 0.8 }),
        color: canvas.dataset.color,
        cell: 3,
      });
    }

    const count = frame.querySelector<HTMLElement>(`[data-channel-count="${channel}"]`);
    if (count) count.textContent = String(members.length);
    frame
      .querySelector<HTMLElement>(`[data-channel="${channel}"]`)
      ?.classList.toggle("is-active", channel === 1 && members.length > 0);
  }
}

let swapAt = 0;
let swapSeed = 2600;
AnimationLoop.shared().add((t) => {
  if (flow.step !== 2 || t < swapAt) return;
  swapAt = t + 4200;
  swapSeed += 1;
  const i = Math.floor(Math.abs(Math.sin(swapSeed * 12.9898)) * 3) % 3;
  assignment[i] = assignment[i] === 1 ? 0 : 1;
  // Never leave the channel empty: an empty channel is a different lesson.
  if (!assignment.includes(1)) assignment[0] = 1;
  renderChannels();
});

/* ---------------------------------------------------------------- step 4 · the timeline */

const timelineCanvas = frame.querySelector<HTMLCanvasElement>('[data-rad-id="timeline"]');
if (timelineCanvas) {
  new TimelineBinding(timelineCanvas, {
    lanes: CAST.map((p) => ({ name: p.name, hue: p.hue })),
  });
}

const recTime = frame.querySelector<HTMLElement>("[data-rectime]");
AnimationLoop.shared().add((t) => {
  if (!recTime || flow.step !== 4) return;
  const seconds = 252 + (Math.floor(t / 1000) % 600);
  recTime.textContent = `${String(Math.floor(seconds / 60)).padStart(2, "0")}:${String(seconds % 60).padStart(2, "0")}`;
});

/* ---------------------------------------------------------------- the gate */

const gateLive = mounted.byId.get("gate-live") as RingBinding | undefined;
if (gateLive) {
  const voices = CAST.map((p, i) => ({ hue: p.hue, phase: i * 1.6 }));
  AnimationLoop.shared().add((t) => {
    gateLive.setSources(
      voices.map((v, i) => ({
        angle: -Math.PI / 2 + i * ((Math.PI * 2) / 3) + Math.sin(t * 0.0005 + i) * 0.4,
        volume: 0.55 + 0.45 * Math.abs(Math.sin(t * 0.0018 + v.phase)),
        hue: v.hue,
      })),
    );
  });
}

/* ---------------------------------------------------------------- sign in
 * The ring is the address resolving. It goes quiet the moment the field changes and
 * comes back when a name resolves, so the visual is a readout rather than decoration.
 */

const lock = mounted.byId.get("lock") as RingBinding | undefined;
const codeRing = mounted.byId.get("code-ring") as RingBinding | undefined;
let resolved = true;
if (lock) {
  AnimationLoop.shared().add((t) => {
    lock.mode = resolved ? "lock" : "empty";
    lock.setSources(
      resolved
        ? [{ angle: -Math.PI / 2 + 0.5, volume: 0.72 + 0.28 * Math.abs(Math.sin(t * 0.0022)), hue: "#ad76f7" }]
        : [],
    );
  });
}

if (codeRing) {
  // Slow, steady, single source: the client is waiting on a person, not on a network.
  AnimationLoop.shared().add((t) => {
    codeRing.setSources([
      { angle: -Math.PI / 2, volume: 0.4 + 0.3 * Math.abs(Math.sin(t * 0.0009)), hue: "#bb8dfa" },
    ]);
  });
}

const HOSTNAME = /^[a-z0-9.-]+\.[a-z]{2,}$/i;
/**
 * The unadvertised way into the code flow.
 *
 * There is no button for it on purpose: a visible "use a sign-in code instead" sends
 * people down the slower path when the fast one would have worked. Typing
 * `code@<server>` is a thing you do because someone told you to, which is exactly the
 * situation the code flow is for — a console player, or a machine with no browser.
 */
const CODE_PREFIX = /^code@(.+)$/i;

const address = frame.querySelector<HTMLInputElement>("[data-address]");
const resolve = frame.querySelector<HTMLElement>("[data-resolve]");
const lockValue = frame.querySelector<HTMLElement>("[data-lock-value]");
let debounce: ReturnType<typeof setTimeout> | undefined;

/** Four letters and four digits: readable aloud, and no ambiguous glyphs. */
function makeCode(): string {
  const letters = "ABCDEFGHJKLMNPQRSTUVWXYZ";
  const pick = (set: string) => set[Math.floor(Math.random() * set.length)];
  return `${[...Array(4)].map(() => pick(letters)).join("")}‑${[...Array(4)]
    .map(() => pick("23456789"))
    .join("")}`;
}

function openCodeScreen(host: string): void {
  const code = frame.querySelector<HTMLElement>("[data-code]");
  if (code) code.textContent = makeCode();
  const server = frame.querySelector<HTMLElement>("[data-code-server]");
  if (server) server.textContent = host;
  const expiry = frame.querySelector<HTMLElement>("[data-code-expiry]");
  if (expiry) expiry.textContent = "○ Expires in 10:00";
  router.go("code");
}

address?.addEventListener("input", () => {
  const code = CODE_PREFIX.exec(address.value.trim());
  if (code) {
    resolved = true;
    if (resolve) {
      resolve.className = "rad-resolve rad-resolve--ok";
      resolve.textContent = "● Sign-in code requested";
    }
    clearTimeout(debounce);
    debounce = setTimeout(() => openCodeScreen(code[1]), 600);
    return;
  }
  resolved = false;
  if (resolve) {
    resolve.className = "rad-resolve";
    resolve.textContent = "○ Resolving";
  }
  if (lockValue) lockValue.textContent = "RESOLVING";
  clearTimeout(debounce);
  const value = address.value.trim();
  debounce = setTimeout(() => {
    const ok = HOSTNAME.test(value);
    resolved = ok;
    if (resolve) {
      resolve.className = `rad-resolve rad-resolve--${ok ? "ok" : "bad"}`;
      resolve.textContent = ok ? "● Resolved · 41 ms" : "✕ Nothing at that address";
    }
    if (lockValue) lockValue.textContent = ok ? "RESOLVED · 41 MS" : "NO RESPONSE";
  }, 700);
});

frame.querySelector<HTMLElement>("[data-connect]")?.addEventListener("click", () => {
  if (resolve) {
    resolve.className = "rad-resolve rad-resolve--ok";
    resolve.textContent = "● Handing off";
  }
  setTimeout(() => Toast.show("Opens the Microsoft sign-in window"), 450);
});

/* ---------------------------------------------------------------- flow and routing */

const flow = new StepFlow({
  frame,
  captions: [
    "Four things worth knowing before you sign in",
    "Proximity keeps running underneath channels",
    "No special hosting, no separate accounts",
    "Everything here works on desktop and phone",
  ],
  onStep: (step) => {
    if (step === 2) renderChannels();
    IconBinding.sync(frame);
  },
  onFinish: () => router.go("gate"),
});

const router = new ScreenRouter(frame, (name) => {
  for (const b of document.querySelectorAll<HTMLElement>("[data-screen]")) {
    b.setAttribute("aria-pressed", String(b.dataset.screen === name));
  }
  if (name === "intro") flow.go(1);
});

for (const button of document.querySelectorAll<HTMLElement>("[data-screen]")) {
  button.addEventListener("click", () => router.go(button.dataset.screen ?? ""));
}

/**
 * What "Copy message" actually copies.
 *
 * Written to be forwarded verbatim to someone who has not heard of BVC: it says what is
 * being asked for, how long it takes, and what to send back. A link on its own gets
 * ignored.
 */
const INVITE_MESSAGE = [
  "Hey — I'd like to use proximity voice chat on our Minecraft world.",
  "",
  "It's a one-time setup on your side: run the BVC server on any machine you control,",
  "and add a small mod to the world. The guide walks through both:",
  "https://wiki.bedrockvoicechat.com/install",
  "",
  "Takes about fifteen minutes. Once it's running, send me the server address and",
  "everyone signs in with the account they already play on.",
].join("\n");

for (const button of document.querySelectorAll<HTMLElement>("[data-toast]")) {
  button.addEventListener("click", () => {
    if (button.dataset.toast?.startsWith("Message copied")) {
      void navigator.clipboard?.writeText(INVITE_MESSAGE).catch(() => {});
    }
    Toast.show(button.dataset.toast ?? "");
  });
}

renderChannels();
