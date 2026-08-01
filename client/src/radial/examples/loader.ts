import { CanvasRecorder } from "$radial/core/intro/CanvasRecorder";
import type { IntroConfig } from "$radial/core/intro/IntroConfig";
import { IntroSequence } from "$radial/core/intro/IntroSequence";
import { Visibility } from "$radial/core/canvas/Visibility";
import { Gallery } from "./gallery";

Gallery.nav();

const el = <T extends HTMLElement>(id: string): T => {
  const found = document.getElementById(id);
  if (!found) throw new Error(`loader: missing #${id}`);
  return found as T;
};

const canvas = el<HTMLCanvasElement>("lo-canvas");
const stage = el("lo-stage");
const status = el("lo-status");
const scrub = el<HTMLInputElement>("scrub");
const timeLabel = el("lo-time");
const playPause = el<HTMLButtonElement>("lo-playpause");

const intro = new IntroSequence(canvas);
let scrubbing = false;
let recording = false;

const cycle = () => intro.sequenceLength + intro.config.holdSeconds;
const format = (t: number) => `${t.toFixed(2)}s / ${cycle().toFixed(2)}s`;

/* ---- fit the canvas into the stage ----
 * The canvas is always at its true export size; only its CSS transform scales it down,
 * so what is recorded is what the export will be rather than what fits on screen.
 */
function fit(): void {
  const pad = 48;
  const k = Math.min(
    1,
    (stage.clientWidth - pad) / intro.config.width,
    (stage.clientHeight - pad) / intro.config.height,
  );
  canvas.style.transform = `scale(${k})`;
  el("lo-fit").textContent =
    `${intro.config.width}×${intro.config.height} · ${Math.round(k * 100)}% preview`;
}
window.addEventListener("resize", fit);

/* ---- transport ---- */
intro.onFrame = (t) => {
  if (scrubbing) return;
  const clamped = Math.min(t, cycle());
  scrub.value = String(clamped);
  timeLabel.textContent = format(clamped);
};

function setPlaying(playing: boolean): void {
  playPause.textContent = playing ? "Pause" : "Play";
}

function play(from = Number(scrub.value)): void {
  intro.start(from >= cycle() - 0.01 ? 0 : from);
  setPlaying(true);
}

function pause(): void {
  intro.seek(Number(scrub.value));
  setPlaying(false);
}

playPause.addEventListener("click", () => (intro.isPaused ? play() : pause()));
el("lo-replay").addEventListener("click", () => {
  scrub.value = "0";
  play(0);
});

const syncScrubRange = () => {
  scrub.max = cycle().toFixed(2);
};

scrub.addEventListener("pointerdown", () => {
  scrubbing = true;
  intro.seek(Number(scrub.value));
  setPlaying(false);
});
scrub.addEventListener("pointerup", () => {
  scrubbing = false;
});
scrub.addEventListener("input", () => {
  const t = Number(scrub.value);
  intro.seek(t);
  timeLabel.textContent = format(t);
  setPlaying(false);
});

/* ---- sliders declare their config key in the markup ---- */
for (const input of document.querySelectorAll<HTMLInputElement>("input[data-key]")) {
  const key = input.dataset.key as keyof IntroConfig;
  const unit = input.dataset.unit ?? "";
  const readout = document.querySelector<HTMLElement>(`[data-val="${key}"]`);
  const paint = () => {
    if (readout) readout.textContent = `${input.value}${unit}`;
  };
  input.addEventListener("input", () => {
    intro.update({ [key]: Number(input.value) } as Partial<IntroConfig>);
    paint();
    if (key === "holdSeconds") syncScrubRange();
    if (key === "scale") fit();
  });
  paint();
}

for (const input of document.querySelectorAll<HTMLInputElement>("input[data-flag]")) {
  input.addEventListener("change", () => {
    intro.update({ [input.dataset.flag as string]: input.checked } as Partial<IntroConfig>);
  });
}

for (const input of document.querySelectorAll<HTMLInputElement>("input[data-color]")) {
  input.addEventListener("input", () => {
    intro.update({ [input.dataset.color as string]: input.value } as Partial<IntroConfig>);
  });
}

/* ---- segmented groups ---- */
function segment(host: HTMLElement, onPick: (button: HTMLButtonElement) => void): void {
  const buttons = [...host.querySelectorAll<HTMLButtonElement>("button")];
  for (const button of buttons) {
    button.addEventListener("click", () => {
      for (const b of buttons) b.setAttribute("aria-pressed", String(b === button));
      onPick(button);
    });
  }
}

const endHint = el("lo-end-hint");
segment(el("lo-end"), (button) => {
  const mode = button.dataset.end === "dance" ? "dance" : "flat";
  intro.update({ endState: mode });
  endHint.textContent =
    mode === "dance"
      ? "Mark keeps dancing at idle gain: the in-app loading indicator."
      : "Mark collapses to its mid row, still in spectrum colour.";
});

const bgCustom = el<HTMLInputElement>("lo-bg-color");
function setBackground(value: string | null): void {
  intro.update({ background: value });
  stage.classList.toggle("is-alpha", value === null);
  if (value) bgCustom.value = value;
}
segment(el("lo-bg"), (button) => {
  const v = button.dataset.bg ?? "";
  setBackground(v === "transparent" ? null : v);
});
bgCustom.addEventListener("input", () => {
  for (const b of el("lo-bg").querySelectorAll("button")) b.setAttribute("aria-pressed", "false");
  setBackground(bgCustom.value);
});

const tint = el<HTMLInputElement>("lo-tint");
segment(el("lo-mark"), (button) => {
  intro.update({ markTint: button.dataset.mark === "single" ? tint.value : null });
});
tint.addEventListener("input", () => {
  const buttons = [...el("lo-mark").querySelectorAll<HTMLButtonElement>("button")];
  buttons.forEach((b, i) => b.setAttribute("aria-pressed", String(i === 1)));
  intro.update({ markTint: tint.value });
});

const widthInput = el<HTMLInputElement>("lo-w");
const heightInput = el<HTMLInputElement>("lo-h");
function applySize(w: number, h: number): void {
  intro.update({ width: w, height: h });
  widthInput.value = String(w);
  heightInput.value = String(h);
  fit();
  if (intro.isPaused) intro.seek(Number(scrub.value));
}
segment(el("lo-size"), (button) => applySize(Number(button.dataset.w), Number(button.dataset.h)));
for (const input of [widthInput, heightInput]) {
  input.addEventListener("change", () => {
    for (const b of el("lo-size").querySelectorAll("button")) b.setAttribute("aria-pressed", "false");
    applySize(Number(widthInput.value), Number(heightInput.value));
  });
}

/* ---- loader preview ---- */
el("lo-loader").addEventListener("click", () => {
  for (const b of el("lo-end").querySelectorAll<HTMLButtonElement>("button")) {
    b.setAttribute("aria-pressed", String(b.dataset.end === "dance"));
  }
  endHint.textContent = "Mark keeps dancing at idle gain: the in-app loading indicator.";
  el<HTMLInputElement>("lo-loop").checked = false;
  intro.startLoading();
  setPlaying(true);
  status.textContent = "Loader preview. Finish loading collapses the mark to its flat row.";
});

el("lo-finish").addEventListener("click", async () => {
  status.textContent = "Collapsing the mark…";
  await intro.finishCollapse();
  status.textContent = "Loading finished: flat colour row, ring still turning.";
});

/* ---- capture ---- */
el("lo-png").addEventListener("click", async () => {
  const t = intro.isPaused ? Number(scrub.value) : intro.currentTime;
  const blob = await CanvasRecorder.toPng(canvas);
  CanvasRecorder.download(blob, `radial-intro-${t.toFixed(2)}s.png`);
  status.textContent = `Saved frame at ${t.toFixed(2)}s.`;
});

const webm = el<HTMLButtonElement>("lo-webm");
webm.addEventListener("click", async () => {
  if (recording) return;
  recording = true;
  webm.disabled = true;
  const wasLooping = intro.config.loop;
  intro.update({ loop: false });
  const seconds = cycle() / intro.config.speed + 0.2;
  status.textContent = `Recording ${seconds.toFixed(1)}s…`;
  play(0);
  try {
    const blob = await CanvasRecorder.toWebM(canvas, seconds);
    CanvasRecorder.download(blob, "radial-intro.webm");
    status.textContent = `Recorded ${(blob.size / 1e6).toFixed(1)} MB WebM.`;
  } catch {
    status.textContent = "Recording failed. Screen capture the stage instead.";
  }
  intro.update({ loop: wasLooping });
  webm.disabled = false;
  recording = false;
});

/* ---- keys ---- */
window.addEventListener("keydown", (e) => {
  const typing = (e.target as HTMLElement)?.tagName === "INPUT";
  if (e.key === " ") {
    e.preventDefault();
    intro.isPaused ? play() : pause();
  } else if ((e.key === "r" || e.key === "R") && !typing) {
    scrub.value = "0";
    play(0);
  } else if (e.key === "ArrowLeft" || e.key === "ArrowRight") {
    if (typing) return;
    e.preventDefault();
    const step = (e.key === "ArrowRight" ? 1 : -1) / 30;
    const t = Math.max(0, Math.min(cycle(), Number(scrub.value) + step));
    scrub.value = String(t);
    intro.seek(t);
    timeLabel.textContent = format(t);
    setPlaying(false);
  }
});

/* ---- boot ---- */
syncScrubRange();
fit();
if (Visibility.prefersReducedMotion()) {
  // Land on the end state rather than animating to it, but leave the transport so the
  // sequence can still be inspected deliberately.
  scrub.value = cycle().toFixed(2);
  intro.seek(cycle());
  setPlaying(false);
  status.textContent = "Reduced motion is on. Press Play to run the sequence.";
} else {
  play(0);
}
