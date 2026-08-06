/**
 * The boot overlay from `app.html`, and taking it down.
 *
 * The overlay is markup outside the bundle, so it stays on screen until something in
 * the app removes it. Every route that renders a first screen has to do that — a route
 * that forgets shows the overlay forever, with its own screen alive and invisible
 * underneath, which reads as the app hanging rather than as a missing call.
 */
export default class BootOverlay {
    private static readonly SELECTOR = '.app-preloader';
    private static readonly HOLD_PARAM = 'preloader-hold';
    /** Long enough that a fast route does not flash the overlay away mid-paint. */
    private static readonly FADE_DELAY_MS = 150;
    /** Defined in static/preloader/app-preloader.css, beside the overlay's own styles. */
    private static readonly FADE_CLASS = 'app-preloader--leaving';
    /** Slightly past the 500ms fade: removing earlier cuts the animation short. */
    private static readonly REMOVE_AFTER_MS = 600;

    /**
     * Fade the overlay out and remove it. Returns false when there was nothing to do,
     * which is the normal answer on a client-side navigation.
     */
    static dismiss(): boolean {
        // Diagnostic hold: `?preloader-hold` keeps the overlay up as if the page never
        // finished initializing, so the stuck-loading escape hatch can be exercised
        // without wedging a real subsystem.
        if (new URLSearchParams(window.location.search).has(BootOverlay.HOLD_PARAM)) {
            return false;
        }

        const overlay = document.querySelector(BootOverlay.SELECTOR);
        if (!overlay) return false;

        setTimeout(() => {
            overlay.classList.add(BootOverlay.FADE_CLASS);
            setTimeout(() => overlay.remove(), BootOverlay.REMOVE_AFTER_MS);
        }, BootOverlay.FADE_DELAY_MS);

        return true;
    }
}
