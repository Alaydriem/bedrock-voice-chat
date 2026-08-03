import { GlyphBinding } from "$radial/bindings/GlyphBinding";
import { IconBinding } from "$radial/bindings/IconBinding";
import { Modal } from "$radial/core/controllers/Modal";
import { Toast } from "$radial/core/controllers/Toast";
import { Color } from "$radial/core/color/Color";
import { ServerGlyph } from "$radial/core/glyph/ServerGlyph";
import { Gallery } from "./gallery";

Gallery.boot();

const frame = Gallery.requireElement("srv-frame");
const modal = new Modal(frame);
const toast = new Toast(frame);

const q = <T extends HTMLElement>(sel: string): T => Gallery.require<T>(frame, sel);

/* ------------------------------------------------------------------ the model */

/** The four the client can already be in, plus the one the UDP probe adds. */
type Status = "checking" | "connect" | "reauth" | "version_mismatch" | "udp_blocked";

type Severity = "ok" | "warn" | "bad";

type StepState = "pending" | "running" | "ok" | "warn" | "bad" | "skipped";

interface Step {
  name: string;
  state: StepState;
  note: string;
  ms: number;
}

interface Server {
  host: string;
  name: string;
  player: string;
  rtt: number;
  quicPort: number;
  serverProtocol: string;
  /** What this server's preflight is scripted to conclude. */
  outcome: Exclude<Status, "checking">;
  /** True when the round trip is high enough to be worth saying out loud. */
  slow: boolean;
  /** Operator-supplied assets. Either can be absent, so neither can be relied on. */
  hasAvatar: boolean;
  hasCanvas: boolean;
  status: Status;
  steps: Step[];
  /** Glyph reveal, 0 to 1. Below 1 the list assembles rather than appears. */
  progress: number;
}

const CLIENT_PROTOCOL = "2.1.0";

const SEED: readonly Omit<Server, "status" | "steps" | "progress">[] = [
  {
    host: "bvc.alaydriem.com",
    name: "Alaydriem's SMP",
    player: "Alaydriem",
    rtt: 24,
    quicPort: 443,
    serverProtocol: "2.1.0",
    outcome: "connect",
    slow: false,
    hasAvatar: true,
    hasCanvas: true,
  },
  {
    host: "voice.hearthhold.net",
    name: "Hearthhold",
    player: "Petra",
    rtt: 38,
    quicPort: 443,
    serverProtocol: "2.1.0",
    outcome: "udp_blocked",
    slow: false,
    hasAvatar: false,
    hasCanvas: true,
  },
  {
    host: "bvc.tinyaxolotl.gg",
    name: "Tiny Axolotl",
    player: "Juno",
    rtt: 46,
    quicPort: 443,
    serverProtocol: "2.1.0",
    outcome: "reauth",
    slow: false,
    hasAvatar: false,
    hasCanvas: false,
  },
  {
    host: "voice.copperandcobble.eu",
    name: "Copper & Cobble",
    player: "Kestrel",
    rtt: 186,
    quicPort: 8443,
    serverProtocol: "2.1.0",
    outcome: "connect",
    slow: true,
    hasAvatar: false,
    hasCanvas: false,
  },
  {
    host: "bvc.deepslate.dev",
    name: "Deepslate",
    player: "Moth",
    rtt: 31,
    quicPort: 443,
    serverProtocol: "2.2.0",
    outcome: "version_mismatch",
    slow: false,
    hasAvatar: true,
    hasCanvas: false,
  },
];

const SERVERS: Server[] = SEED.map((s) => ({ ...s, status: "checking", steps: [], progress: 0 }));

const state = { count: 5 };

/** Only `version_mismatch` keeps a lone server on this page today, so that is the one. */
function visible(): Server[] {
  if (state.count === 1) return SERVERS.filter((s) => s.outcome === "version_mismatch");
  return SERVERS.slice(0, state.count);
}

/* ------------------------------------------------------------------ the checks
 * Scripted rather than random: the point of the panel is that a specific failure
 * produces a specific sentence, and a demo that jitters around healthy numbers never
 * shows the states that matter.
 *
 * Certificate expiry is checked but never shown. It is the mechanism behind "you are
 * not signed in any more", and naming it asks a player to care about mTLS to
 * understand that they need to sign in again.
 */

const STEP_NAMES = ["Credentials", "Handshake", "Protocol", "QUIC path"] as const;

function pendingSteps(): Step[] {
  return STEP_NAMES.map((name) => ({ name, state: "pending", note: "", ms: 0 }));
}

function jitter(base: number, spread: number): number {
  return Math.max(1, Math.round(base + (Math.random() - 0.5) * spread));
}

/** The result of one step, given the server it ran against. */
function resolve(s: Server, index: number): Step {
  const name = STEP_NAMES[index];
  switch (index) {
    case 0:
      return s.outcome === "reauth"
        ? { name, state: "bad", note: "no valid sign-in for this server", ms: jitter(18, 8) }
        : { name, state: "ok", note: `signed in as ${s.player}`, ms: jitter(18, 8) };
    case 1:
      return {
        name,
        state: s.slow ? "warn" : "ok",
        note: `mTLS · TLS 1.3 · ${s.rtt} ms round trip`,
        ms: jitter(s.rtt * 1.8, s.rtt * 0.4),
      };
    case 2:
      return {
        name,
        state: s.outcome === "version_mismatch" ? "bad" : "ok",
        note:
          s.outcome === "version_mismatch"
            ? `client ${CLIENT_PROTOCOL} · server ${s.serverProtocol} — client is too old`
            : `${CLIENT_PROTOCOL} · server ${s.serverProtocol}`,
        ms: jitter(s.rtt * 0.9, s.rtt * 0.3),
      };
    default:
      return {
        name,
        state: s.outcome === "udp_blocked" ? "bad" : "ok",
        note:
          s.outcome === "udp_blocked"
            ? `udp/${s.quicPort} unreachable · 3 probes, no response`
            : `udp/${s.quicPort} open · ${jitter(s.rtt + 4, 6)} ms` +
              (s.quicPort === 443 ? "" : " · fallback port"),
        ms: s.outcome === "udp_blocked" ? jitter(1500, 120) : jitter(s.rtt + 12, 14),
      };
  }
}

/** A failing check stops the sequence: the ones after it never ran and say so. */
function statusFrom(steps: Step[], outcome: Server["outcome"]): Status {
  if (steps.some((step) => step.state === "pending" || step.state === "running")) return "checking";
  return outcome;
}

/* Timers are held per server so rechecking one does not cancel the other four. */
const timers = new Map<string, Set<number>>();

function later(host: string, fn: () => void, delay: number): void {
  const set = timers.get(host) ?? new Set<number>();
  timers.set(host, set);
  set.add(window.setTimeout(fn, delay));
}

function cancel(host: string): void {
  for (const t of timers.get(host) ?? []) window.clearTimeout(t);
  timers.delete(host);
}

/**
 * Run one server's preflight: concurrent with every other server, sequential in its
 * own steps. That is what the client does, and it is why the list resolves in a
 * ragged order rather than top to bottom.
 */
function runFor(s: Server, offset = 0, reveal = true): void {
  cancel(s.host);
  s.status = "checking";
  s.steps = pendingSteps();
  if (reveal) s.progress = 0;
  paint(s);

  let clock = 80 + offset;
  const step = (index: number): void => {
    if (index >= STEP_NAMES.length) return;
    const result = resolve(s, index);
    later(
      s.host,
      () => {
        s.steps[index] = { ...result, state: "running", note: "", ms: 0 };
        paint(s);
      },
      clock,
    );
    clock += Math.min(result.ms, 620);
    later(
      s.host,
      () => {
        s.steps[index] = result;
        if (result.state === "bad") {
          for (let k = index + 1; k < s.steps.length; k++) {
            s.steps[k] = { name: STEP_NAMES[k], state: "skipped", note: "not run", ms: 0 };
          }
        }
        s.status = statusFrom(s.steps, s.outcome);
        paint(s);
        if (result.state !== "bad") step(index + 1);
      },
      clock,
    );
  };

  step(0);
  if (reveal) revealGlyph(s, 420 + offset);
}

function runAll(): void {
  for (const s of SERVERS) {
    cancel(s.host);
    s.status = "checking";
    s.steps = pendingSteps();
    s.progress = 0;
  }
  render();
  // A small per-server offset so five identical spinners do not pulse in lockstep.
  for (const [i, s] of visible().entries()) runFor(s, i * 90);
}

/* --------------------------------------------------------------- the sentence */

/**
 * First failing check wins, matching `Diagnostics.verdict`.
 *
 * Somebody looking at this has a problem. A summary that averages four results into
 * "mostly fine" leaves them to find the one line that is not.
 */
function verdict(s: Server): readonly [Severity, string] {
  if (s.status === "checking") return ["ok", "Checking…"];
  const failed = s.steps.find((step) => step.state === "bad");
  if (failed) {
    switch (failed.name) {
      case "Credentials":
        return ["bad", "You are not signed in to this server any more. Sign in again to connect."];
      case "Protocol":
        return [
          "bad",
          `This server speaks protocol ${s.serverProtocol} and your client speaks ` +
            `${CLIENT_PROTOCOL}. Update the client to connect.`,
        ];
      default:
        return [
          "bad",
          `UDP ${s.quicPort} to this server is blocked, so voice cannot connect at all. ` +
            "Check the network or firewall you are behind, then recheck.",
        ];
    }
  }
  const warned = s.steps.find((step) => step.state === "warn");
  if (warned) {
    return ["warn", `${s.rtt} ms to this server. Voice works, with delay you will notice.`];
  }
  return ["ok", "Everything looks fine."];
}

/** What the primary button does. A blocked server never offers a doomed connect. */
type ActionKind = "connect" | "signin" | "recheck" | "blocked";

interface Presentation {
  chip: string;
  severity: Severity | "muted";
  action: string;
  kind: ActionKind;
}

function present(s: Server): Presentation {
  switch (s.status) {
    case "connect":
      return s.slow
        ? { chip: "Ready · slow", severity: "warn", action: "Connect", kind: "connect" }
        : { chip: "Ready", severity: "ok", action: "Connect", kind: "connect" };
    case "reauth":
      return { chip: "Sign in needed", severity: "bad", action: "Sign in", kind: "signin" };
    case "version_mismatch":
      return {
        chip: "Update needed",
        severity: "warn",
        action: `Client ${CLIENT_PROTOCOL} → ${s.serverProtocol}`,
        kind: "blocked",
      };
    case "udp_blocked":
      // Connecting would fail, so recheck is the only thing worth offering.
      return { chip: "Voice blocked", severity: "bad", action: "Recheck", kind: "recheck" };
    default:
      return { chip: "Checking…", severity: "muted", action: "Connect", kind: "blocked" };
  }
}

/* ------------------------------------------------------------------- the art
 * Generated from each server's own hue. The reference pages reach nothing over the
 * network, and a placeholder in a foreign palette would misrepresent how the plate
 * looks once real art lands in it.
 */

function artFor(host: string): string {
  const glyph = ServerGlyph.of(host);
  let seed = 0;
  for (let i = 0; i < host.length; i++) seed = (seed * 31 + host.charCodeAt(i)) >>> 0;
  const next = (): number => {
    seed = (seed * 1664525 + 1013904223) >>> 0;
    return seed / 4294967296;
  };

  const bands = Array.from({ length: 5 }, (_, i) => {
    const y = 12 + i * 22 + next() * 8;
    const h = 10 + next() * 26;
    return `<rect x='-20' y='${y.toFixed(1)}' width='360' height='${h.toFixed(1)}' fill='${
      glyph.hue
    }' opacity='${(0.14 + next() * 0.3).toFixed(2)}' transform='rotate(-4 160 60)'/>`;
  }).join("");

  const blocks = Array.from({ length: 22 }, () => {
    const size = 5 + Math.round(next() * 9);
    return `<rect x='${(next() * 320).toFixed(0)}' y='${(next() * 120).toFixed(0)}' width='${size}' height='${size}' fill='${
      next() > 0.55 ? "#ffffff" : glyph.hue
    }' opacity='${(0.05 + next() * 0.16).toFixed(2)}'/>`;
  }).join("");

  const svg =
    `<svg xmlns='http://www.w3.org/2000/svg' width='320' height='120'>` +
    `<rect width='320' height='120' fill='#1c1132'/>` +
    `<rect width='320' height='120' fill='${glyph.hue}' opacity='0.2'/>` +
    bands +
    blocks +
    `</svg>`;
  return `url("data:image/svg+xml,${encodeURIComponent(svg)}")`;
}

/** `avatar.png`, when the operator supplied one. Their mark, not ours, so it is flat. */
function avatarFor(host: string): string {
  const glyph = ServerGlyph.of(host);
  const svg =
    `<svg xmlns='http://www.w3.org/2000/svg' width='64' height='64'>` +
    `<rect width='64' height='64' fill='${glyph.hue}'/>` +
    `<circle cx='32' cy='27' r='13' fill='${Color.rgba("#ffffff", 0.86)}'/>` +
    `<path d='M8 64c0-13 11-21 24-21s24 8 24 21z' fill='${Color.rgba("#ffffff", 0.86)}'/>` +
    `</svg>`;
  return `data:image/svg+xml,${encodeURIComponent(svg)}`;
}

/* ------------------------------------------------------------------- the plate */

const list = q("[data-s-list]");

function identityHtml(host: string, hasAvatar: boolean, size: number, extraClass = ""): string {
  const inner = hasAvatar
    ? `<img src="${avatarFor(host)}" alt="" />`
    : `<canvas data-s-glyph="${host}" width="${size}" height="${size}"></canvas>`;
  return `<span class="rad-server-id ${extraClass}" style="width:${size}px;height:${size}px">${inner}</span>`;
}

/** The head of a plate: operator art when it exists, the derived hue when it does not. */
function artHtml(host: string, hasCanvas: boolean, hasAvatar: boolean): string {
  const open = hasCanvas
    ? `<span class="rad-server__art" style="background-image:${artFor(host)}">`
    : `<span class="rad-server__art rad-server__art--derived" style="--rad-server-hue:${
        ServerGlyph.of(host).hue
      }">`;
  return `${open}${identityHtml(host, hasAvatar, 52, "rad-server__id")}</span>`;
}

function chipHtml(p: Presentation): string {
  return `<span class="rad-status-chip rad-status-chip--${p.severity}">${p.chip}</span>`;
}

function preflightHtml(s: Server): string {
  const dots = s.steps.length ? s.steps : pendingSteps();
  const cls = (step: Step): string =>
    step.state === "running"
      ? "is-run"
      : step.state === "skipped"
        ? "is-skip"
        : step.state === "pending"
          ? ""
          : `is-${step.state}`;
  const total = dots.reduce((sum, step) => sum + step.ms, 0);
  const label = s.status === "checking" ? "checking" : `${total} ms`;
  return (
    `<button class="rad-preflight" data-s-open="${s.host}" title="Open the preflight readout">` +
    `<span class="rad-preflight__blocks">${dots.map((step) => `<i class="${cls(step)}"></i>`).join("")}</span>` +
    `<span class="rad-preflight__total">${label}</span>` +
    `</button>`
  );
}

function actionHtml(s: Server, p: Presentation): string {
  const blocked = p.kind === "blocked";
  const label = blocked && s.status === "version_mismatch" ? "Update" : p.action;
  return (
    `<button class="rad-btn${blocked ? "" : " rad-btn--primary"}" data-s-go="${s.host}"` +
    `${blocked ? " disabled" : ""}>${label}</button>`
  );
}

function plateHtml(s: Server, i: number): string {
  const p = present(s);
  return (
    `<div class="rad-server rad-rise" style="--d:${i * 60}" data-s-card="${s.host}">` +
    artHtml(s.host, s.hasCanvas, s.hasAvatar) +
    `<span class="rad-server__body">` +
    `<span class="rad-server__name">${s.name}</span>` +
    `<span class="rad-server__host">${s.host}</span>` +
    `<span class="rad-server__state">${chipHtml(p)}</span>` +
    `</span>` +
    `<span class="rad-server__foot">${preflightHtml(s)}${actionHtml(s, p)}</span>` +
    `</div>`
  );
}

const addHtml =
  `<button class="rad-server-add" data-s-add>` +
  `<span data-rad-icon="plus"></span>` +
  `<span class="rad-server-add__label">Add a server</span>` +
  `</button>`;

function render(): void {
  const servers = visible();
  list.innerHTML =
    `<div class="rad-server-grid">` + servers.map(plateHtml).join("") + addHtml + `</div>`;
  hydrate(list);
  wireCards();
  paintAggregate();
  q("[data-s-count-out]").textContent =
    `${servers.length} ${servers.length === 1 ? "server" : "servers"}`;
}

/** Repaint one server in place, so a resolving check does not rebuild the list. */
function paint(s: Server): void {
  const card = list.querySelector<HTMLElement>(`[data-s-card="${s.host}"]`);
  if (card) {
    const p = present(s);
    const chip = card.querySelector<HTMLElement>(".rad-status-chip");
    if (chip) {
      chip.className = `rad-status-chip rad-status-chip--${p.severity}`;
      chip.textContent = p.chip;
    }
    const pf = card.querySelector<HTMLElement>(".rad-preflight");
    if (pf) pf.outerHTML = preflightHtml(s);
    const go = card.querySelector<HTMLButtonElement>("[data-s-go]");
    if (go) go.outerHTML = actionHtml(s, p);
    wireCards();
  }
  paintAggregate();
  if (modal.openName === "preflight" && detailHost === s.host) renderDetail(s);
}

function paintAggregate(): void {
  const tally = new Map<string, { n: number; colour: string }>();
  const add = (label: string, colour: string) => {
    const row = tally.get(label) ?? { n: 0, colour };
    row.n += 1;
    tally.set(label, row);
  };
  for (const s of visible()) {
    switch (s.status) {
      case "connect":
        add(s.slow ? "slow" : "ready", s.slow ? "#ffcf4d" : "#5ce383");
        break;
      case "reauth":
        add("need sign-in", "#ff8266");
        break;
      case "version_mismatch":
        add("outdated", "#ffcf4d");
        break;
      case "udp_blocked":
        add("voice blocked", "#ff8266");
        break;
      default:
        add("checking", "#bb8dfa");
    }
  }
  q("[data-s-agg]").innerHTML = [...tally.entries()]
    .map(
      ([label, row]) =>
        `<span class="rad-server-tally__item"><i style="background:${row.colour}"></i>${row.n} ${label}</span>`,
    )
    .join("");
}

/* --------------------------------------------------------------- the readout */

let detailHost = "";

function kvGroup(title: string, rows: readonly (readonly [string, string])[]): string {
  return (
    `<div class="rad-kv-group"><div class="rad-kv-group__head">${title}</div>` +
    rows
      .map(
        ([k, v]) =>
          `<div class="rad-kv"><span class="rad-kv__key">${k}</span>` +
          `<span class="rad-kv__value">${v}</span></div>`,
      )
      .join("") +
    `</div>`
  );
}

function renderDetail(s: Server): void {
  const [severity, sentence] = verdict(s);
  const steps = (s.steps.length ? s.steps : pendingSteps())
    .map((step) => {
      const cls =
        step.state === "running"
          ? "is-run"
          : step.state === "skipped"
            ? "is-skip"
            : step.state === "pending"
              ? "is-pending"
              : `is-${step.state}`;
      const ms = step.state === "skipped" ? "—" : step.ms ? `${step.ms} ms` : "…";
      return (
        `<div class="rad-preflight-step ${cls}"><span class="rad-preflight-step__dot"></span>` +
        `<span><span class="rad-preflight-step__name">${step.name}</span>` +
        `<span class="rad-preflight-step__note">${step.note || "…"}</span></span>` +
        `<span class="rad-preflight-step__ms">${ms}</span></div>`
      );
    })
    .join("");
  const shown = s.steps.length ? s.steps : pendingSteps();
  const total = shown.reduce((sum, step) => sum + step.ms, 0);
  const ran = shown.filter((step) => step.state !== "skipped" && step.state !== "pending").length;

  q("[data-s-detail-name]").textContent = s.name;
  q("[data-s-detail-host]").textContent = s.host;
  q("[data-s-detail-id]").innerHTML = s.hasAvatar
    ? `<img src="${avatarFor(s.host)}" alt="" />`
    : `<canvas data-s-glyph="${s.host}" width="56" height="56"></canvas>`;

  q("[data-s-detail-body]").innerHTML =
    `<div class="rad-verdict rad-verdict--${severity}">` +
    `<span class="rad-verdict__dot"></span>` +
    `<span class="rad-verdict__text">${sentence}</span></div>` +
    `<div class="rad-preflight-steps">${steps}</div>` +
    `<div class="rad-preflight-total"><span>${ran} of ${STEP_NAMES.length} checks ran</span>` +
    `<span><b>${total} ms</b> total</span></div>` +
    `<div class="rad-kv-grid" style="margin-top:22px">` +
    kvGroup("Account", [
      ["Signed in as", s.player],
      ["Game", "Minecraft Bedrock"],
      ["Stored in", "keyring · servers"],
    ]) +
    kvGroup("Link", [
      ["Round trip", `${s.rtt} ms`],
      ["QUIC port", `${s.quicPort}${s.quicPort === 443 ? "" : " (fallback)"}`],
      ["Protocol", `${CLIENT_PROTOCOL} · server ${s.serverProtocol}`],
      ["Transport", "TLS 1.3 · mTLS"],
    ]) +
    `</div>`;

  const p = present(s);
  const go = q<HTMLButtonElement>("[data-s-detail-go]");
  go.textContent = p.kind === "blocked" ? "Update the client" : p.action;
  // Blocked still leads somewhere: the update, not the connection that would fail.
  go.disabled = s.status === "checking";

  hydrate(q("[data-s-detail-body]"));
  const canvas = q("[data-s-detail-id]").querySelector<HTMLCanvasElement>("canvas");
  if (canvas) new GlyphBinding(canvas, s.host, { size: 56 });
}

function openDetail(host: string): void {
  const s = SERVERS.find((entry) => entry.host === host);
  if (!s) return;
  detailHost = host;
  renderDetail(s);
  modal.open("preflight");
}

/* ------------------------------------------------------------------- wiring */

const glyphs = new Map<HTMLCanvasElement, GlyphBinding>();

/** Bindings for markup that was written after `Mount.scan` ran. */
function hydrate(root: ParentNode): void {
  for (const el of root.querySelectorAll<HTMLElement>("[data-rad-icon]")) new IconBinding(el);
  for (const canvas of root.querySelectorAll<HTMLCanvasElement>("canvas[data-s-glyph]")) {
    const host = canvas.dataset.sGlyph ?? "";
    const server = SERVERS.find((s) => s.host === host);
    const binding = new GlyphBinding(canvas, host, { size: canvas.width });
    binding.render(server ? server.progress : 1);
    glyphs.set(canvas, binding);
  }
}

/** The glyph draws in over its own reveal, so the list assembles. */
function revealGlyph(s: Server, duration: number): void {
  const start = performance.now();
  const tick = (now: number): void => {
    s.progress = Math.min(1, (now - start) / duration);
    for (const [canvas, binding] of glyphs) {
      if (canvas.dataset.sGlyph !== s.host) continue;
      if (!canvas.isConnected) {
        glyphs.delete(canvas);
        continue;
      }
      binding.render(s.progress);
    }
    if (s.progress < 1) requestAnimationFrame(tick);
  };
  requestAnimationFrame(tick);
}

function act(s: Server): void {
  const p = present(s);
  switch (p.kind) {
    case "connect":
      toast.show(`/dashboard?server=${s.host}`);
      break;
    case "signin":
      toast.show(`/login?reauth=true&server=${s.host}`);
      break;
    case "recheck":
      runFor(s, 0, false);
      break;
    default:
      toast.show("Blocked until the client is updated");
  }
}

function wireCards(): void {
  for (const el of list.querySelectorAll<HTMLElement>("[data-s-open]")) {
    el.onclick = (e) => {
      e.stopPropagation();
      openDetail(el.dataset.sOpen ?? "");
    };
  }
  for (const el of list.querySelectorAll<HTMLButtonElement>("[data-s-go]")) {
    el.onclick = (e) => {
      e.stopPropagation();
      const s = SERVERS.find((entry) => entry.host === el.dataset.sGo);
      if (s) act(s);
    };
  }
  for (const el of list.querySelectorAll<HTMLElement>("[data-s-add]")) {
    el.onclick = () => toast.show("/login?addserver=true&return=/server");
  }
  for (const card of list.querySelectorAll<HTMLElement>("[data-s-card]")) {
    card.onmouseenter = () => card.classList.add("is-focus");
    card.onmouseleave = () => card.classList.remove("is-focus");
  }
}

for (const el of frame.querySelectorAll<HTMLElement>("[data-s-recheck]")) {
  el.addEventListener("click", () => runAll());
}
for (const el of frame.querySelectorAll<HTMLElement>(".rad-topbar [data-s-add]")) {
  el.addEventListener("click", () => toast.show("/login?addserver=true&return=/server"));
}

q<HTMLButtonElement>("[data-s-detail-recheck]").addEventListener("click", () => {
  const s = SERVERS.find((entry) => entry.host === detailHost);
  if (s) runFor(s, 0, false);
});
q<HTMLButtonElement>("[data-s-detail-remove]").addEventListener("click", () => {
  toast.show("Remove asks for confirmation first");
});
q<HTMLButtonElement>("[data-s-detail-go]").addEventListener("click", () => {
  const s = SERVERS.find((entry) => entry.host === detailHost);
  if (s) act(s);
});

for (const el of document.querySelectorAll<HTMLElement>("[data-s-count]")) {
  el.addEventListener("click", () => {
    state.count = Number.parseInt(el.dataset.sCount ?? "5", 10);
    for (const b of document.querySelectorAll<HTMLElement>("[data-s-count]")) {
      b.setAttribute("aria-pressed", String(b === el));
    }
    runAll();
  });
}

/* ---- the identity specimens, outside the frame ----
 * Real plate heads, so the three asset cases are shown by the component that has to
 * handle them rather than by an illustration of it.
 */

const identity = document.querySelector<HTMLElement>("[data-s-identity]");
if (identity) {
  const cases: readonly (readonly [string, string, boolean, boolean])[] = [
    ["bvc.alaydriem.com", "avatar.png + canvas.png", true, true],
    ["voice.hearthhold.net", "canvas.png only — glyph carries identity", false, true],
    ["bvc.tinyaxolotl.gg", "neither — hue and glyph both derived", false, false],
  ];
  identity.innerHTML = cases
    .map(
      ([host, label, hasAvatar, hasCanvas]) =>
        `<div class="s-spec">` +
        `<div class="rad-server">${artHtml(host, hasCanvas, hasAvatar)}</div>` +
        `<span class="s-spec__caption">` +
        `<span class="rad-server__host">${host}</span>` +
        `<span class="g-tag">${label}</span>` +
        `</span></div>`,
    )
    .join("");
  hydrate(identity);
}

runAll();
