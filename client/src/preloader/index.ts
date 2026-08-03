import { RingBinding } from "$radial/bindings/RingBinding";
import { AnimationLoop } from "$radial/core/canvas/AnimationLoop";
import { Visibility } from "$radial/core/canvas/Visibility";
import { LoaderStatus } from "$radial/core/intro/LoaderStatus";
import { MarkData } from "$radial/core/mark/MarkData";
import type { RingSource } from "$radial/core/ring/RingSource";

/**
 * The boot preloader in `app.html`.
 *
 * Bundled to a standalone IIFE in `static/preloader/` rather than shipped inside the
 * app bundle: when a page hangs during init this is the only script guaranteed to be
 * running, so it cannot depend on the bundle having loaded. It has no runtime
 * imports — everything it uses is compiled in — which is the constraint the previous
 * hand-written version stated and this one keeps.
 *
 * It draws the same thing the introduction's first step does, through the same
 * renderer: a live ring with voices on it and the mark at full amplitude in its own
 * spectrum. Deliberately not the boot sequence, which rests at a third of its gain
 * with the ring drained toward violet-grey — correct as an arrival, but this overlay
 * is a wait, and it was reading as grey circles.
 */

// Routes that can hang before they put a screen up, leaving this overlay as the only
// thing the user can act on. Login is excluded because its connecting view carries its
// own cancel and its own recovery link. Setup carries neither: a failed initialize there
// has no way out except this.
const ESCAPE_HATCH_ROUTES = ["/", "/dashboard", "/settings", "/server", "/setup"];
const ESCAPE_HATCH_DELAY_MS = 10000;

// Staged reveal: the mark shows instantly, the status copy only once a load is slow
// enough for text to be worth reading.
const STATUS_AFTER_SECONDS = 1;
const FRAME_SECONDS = 0.09;
const PHRASE_SECONDS = 1.6;

const STATUS_PHRASES = [
    "Getting things ready…",
    "Connecting to your BVC Server…",
    "Almost there…",
];

// Three voices, hues taken from columns of the mark so the ring is provably part of
// the palette rather than picked next to it. Same cast shape as the gate's live state.
const VOICES = [1, 8, 14].map((column, i) => ({
    hue: MarkData.hueAt(column),
    phase: i * 1.6,
}));

class AppPreloaderController {
    private readonly root: HTMLElement;
    private readonly markEl: HTMLCanvasElement | null;
    private readonly frameEl: HTMLElement | null;
    private readonly headlineEl: HTMLElement | null;
    private readonly statusEl: HTMLElement | null;
    private readonly actionsEl: HTMLElement | null;

    private ring: RingBinding | null = null;
    private stopSources: (() => void) | null = null;
    private stopFrames: (() => void) | null = null;

    constructor(root: HTMLElement) {
        this.root = root;
        this.markEl = root.querySelector<HTMLCanvasElement>("#app-preloader-mark");
        this.frameEl = root.querySelector<HTMLElement>("#app-preloader-frame");
        this.headlineEl = root.querySelector<HTMLElement>("#app-preloader-headline");
        this.statusEl = root.querySelector<HTMLElement>("#app-preloader-status");
        this.actionsEl = root.querySelector<HTMLElement>("#app-preloader-actions");
    }

    private isDismissed(): boolean {
        return !document.body.contains(this.root);
    }

    /**
     * The ring, live, with voices moving on it.
     *
     * No size is passed: `RingBinding` fits itself to the canvas's CSS box every
     * frame, so the stylesheet owns the size and a window resize needs no handling
     * here at all.
     */
    private startMark(): void {
        if (!this.markEl) return;
        this.ring = new RingBinding(this.markEl, { mode: "live" });

        const ring = this.ring;
        this.stopSources = AnimationLoop.shared().add((t) => {
            const sources: RingSource[] = VOICES.map((v, i) => ({
                angle: -Math.PI / 2 + i * ((Math.PI * 2) / 3) + Math.sin(t * 0.0005 + i) * 0.4,
                volume: 0.55 + 0.45 * Math.abs(Math.sin(t * 0.0018 + v.phase)),
                hue: v.hue,
            }));
            ring.setSources(sources);
        });
    }

    /**
     * Braille and status copy, from the kit's own state machine rather than a second
     * implementation of it. Reduced motion holds a single frame instead of cycling.
     */
    private startStatus(): void {
        if (!this.frameEl || !this.statusEl || !this.headlineEl) return;

        const status = new LoaderStatus({
            phrases: STATUS_PHRASES,
            slowAfterSeconds: STATUS_AFTER_SECONDS,
            phraseSeconds: PHRASE_SECONDS,
            frameSeconds: FRAME_SECONDS,
        });

        if (Visibility.prefersReducedMotion()) {
            const settled = status.at(STATUS_AFTER_SECONDS);
            this.frameEl.textContent = settled.glyph;
            this.statusEl.textContent = settled.phrase;
            this.headlineEl.hidden = false;
            this.statusEl.hidden = false;
            return;
        }

        let start: number | null = null;
        this.stopFrames = AnimationLoop.shared().add((t) => {
            if (this.isDismissed()) {
                this.teardown();
                return;
            }
            start ??= t;
            const frame = status.at((t - start) / 1000);
            if (!frame.visible) return;
            this.headlineEl!.hidden = false;
            this.statusEl!.hidden = false;
            this.frameEl!.textContent = frame.glyph;
            this.statusEl!.textContent = frame.phrase;
        });
    }

    private armHatch(): void {
        const path = window.location.pathname.replace(/\/+$/, "") || "/";
        if (!ESCAPE_HATCH_ROUTES.includes(path)) return;

        // Switching servers only makes sense when another server exists, and is
        // circular when the stuck page IS the server picker. The count is mirrored
        // into localStorage by ServerListStore because the Tauri store is unreachable
        // before hydration; unreadable storage counts as zero so only the sign-out
        // escape shows.
        let serverCount = 0;
        try {
            serverCount = parseInt(window.localStorage.getItem("bvc_server_count") ?? "0", 10);
        } catch (_) {}
        if (path === "/server" || !(serverCount > 1)) {
            const serverAction = this.root.querySelector<HTMLElement>(
                "#app-preloader-action-server",
            );
            if (serverAction) serverAction.hidden = true;
        }

        this.root
            .querySelector<HTMLElement>("#app-preloader-keep-waiting")
            ?.addEventListener("click", () => {
                if (this.actionsEl) this.actionsEl.hidden = true;
                if (this.headlineEl) this.headlineEl.textContent = "Loading…";
                this.scheduleReveal();
            });

        this.scheduleReveal();
    }

    private scheduleReveal(): void {
        setTimeout(() => {
            if (this.isDismissed()) return;
            if (this.headlineEl) {
                this.headlineEl.textContent = "This is taking longer than expected.";
                this.headlineEl.hidden = false;
            }
            if (this.actionsEl) this.actionsEl.hidden = false;
        }, ESCAPE_HATCH_DELAY_MS);
    }

    private teardown(): void {
        this.stopFrames?.();
        this.stopFrames = null;
        this.stopSources?.();
        this.stopSources = null;
        this.ring?.destroy();
        this.ring = null;
    }

    start(): void {
        this.startMark();
        this.startStatus();
        this.armHatch();
    }
}

const root = document.querySelector<HTMLElement>(".app-preloader");
if (root) new AppPreloaderController(root).start();
