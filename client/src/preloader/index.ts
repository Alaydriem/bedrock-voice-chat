import { RingBinding } from "$radial/bindings/RingBinding";
import { AnimationLoop } from "$radial/core/canvas/AnimationLoop";
import { Visibility } from "$radial/core/canvas/Visibility";
import { LoaderStatus } from "$radial/core/intro/LoaderStatus";
import { ProximityCast } from "$radial/core/sources/ProximityCast";

/**
 * The boot preloader in `app.html`.
 *
 * Bundled to a standalone IIFE in `static/preloader/` rather than shipped inside the
 * app bundle: when a page hangs during init this is the only script guaranteed to be
 * running, so it cannot depend on the bundle having loaded. It has no runtime
 * imports — everything it uses is compiled in — which is the constraint the previous
 * hand-written version stated and this one keeps.
 *
 * It draws what `ProximityRing` draws in the app — the cast of `ProximityCast` placed on a
 * live ring, with the mark dancing at full amplitude — so the overlay and the first screen
 * it hands over to are the same object. Expressed through `RingBinding` rather than the
 * component, because Svelte is in the bundle this file exists to run without.
 */

// Routes that can hang before they put a screen up, leaving this overlay as the only
// thing the user can act on. Login is excluded because its connecting view carries its
// own cancel and its own recovery link. Setup carries neither: a failed initialize there
// has no way out except this.
const ESCAPE_HATCH_ROUTES = ["/", "/dashboard", "/settings", "/server", "/setup"];
const ESCAPE_HATCH_DELAY_MS = 10000;

// Staged reveal: the mark shows instantly, the status copy only once a load is slow
// enough for text to be worth reading. Three seconds, not one — a normal launch finishes
// inside that, and a line of text that appears and is gone again reads as a glitch rather
// than as an explanation.
const STATUS_AFTER_SECONDS = 3;
const FRAME_SECONDS = 0.09;
const PHRASE_SECONDS = 1.6;

/** Withholds the paint but keeps the box, so revealing these costs no layout. */
const QUIET_CLASS = "app-preloader-quiet";

const STATUS_PHRASES = [
    "Getting things ready…",
    "Connecting to your BVC Server…",
    "Almost there…",
];

/** As many of the cast as `ProximityRing` places in the app. */
const CAST_COUNT = 4;

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
     * The ring, live, with the cast placed on it.
     *
     * No size is passed: `RingBinding` fits itself to the canvas's CSS box every frame, so
     * the stylesheet owns the size and a resize needs no handling here at all. A live ring
     * dances the mark at full amplitude, which is what the app's own loader shows.
     */
    private startMark(): void {
        if (!this.markEl) return;
        this.ring = new RingBinding(this.markEl, { mode: "live" });

        const ring = this.ring;
        this.stopSources = AnimationLoop.shared().add((t) => {
            ring.setSources(ProximityCast.placements(t, CAST_COUNT));
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
            this.reveal();
            return;
        }

        let start: number | null = null;
        let revealed = false;
        let glyph = "";
        let phrase = "";

        this.stopFrames = AnimationLoop.shared().add((t) => {
            if (this.isDismissed()) {
                this.teardown();
                return;
            }
            start ??= t;
            const frame = status.at((t - start) / 1000);
            if (!frame.visible) return;

            if (!revealed) {
                revealed = true;
                this.reveal();
            }

            /*
             * Written only when they change.
             *
             * The glyph turns eleven times a second and the phrase every 1.6, on a loop
             * that runs sixty times a second — so all but a fraction of these assignments
             * set the text to what it already said. They were not free: assigning
             * `textContent` invalidates layout, and `Surface.fit` measures the canvas with
             * `getBoundingClientRect` on the very next callback, which turns an invalidated
             * layout into a synchronous reflow. One per frame, on the screen that is up
             * precisely because the device is already busy.
             */
            if (frame.glyph !== glyph) {
                glyph = frame.glyph;
                this.frameEl!.textContent = glyph;
            }
            if (frame.phrase !== phrase) {
                phrase = frame.phrase;
                this.statusEl!.textContent = phrase;
            }
        });
    }

    /** Paints what was already holding its place in the layout. */
    private reveal(): void {
        this.frameEl?.classList.remove(QUIET_CLASS);
        this.statusEl?.classList.remove(QUIET_CLASS);
        this.headlineEl?.classList.remove(QUIET_CLASS);
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
                this.scheduleHatch();
            });

        this.scheduleHatch();
    }

    /**
     * Unlike the status line, this one is meant to change the layout: it is a different
     * state with something to act on, not the same wait with a caption.
     */
    private scheduleHatch(): void {
        setTimeout(() => {
            if (this.isDismissed()) return;
            if (this.headlineEl) {
                this.headlineEl.textContent = "This is taking longer than expected.";
                this.headlineEl.classList.remove(QUIET_CLASS);
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
