/* Drives the boot preloader in app.html: braille frames from JS (mirroring
   the login and onboarding loaders) and escape actions revealed when a page's
   init never dismisses the overlay. Served as a static asset rather than
   bundled because it must run before the SvelteKit bundle hydrates - when a
   page hangs during init, this is the only script guaranteed to be running.
   It must stay dependency-free: no imports, no Tailwind classes (it toggles
   the native hidden attribute), no Tauri APIs. */
(() => {
  const BRAILLE_FRAMES = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
  const FRAME_INTERVAL_MS = 90;
  // Routes whose init depends on an active session and can hang; the hatch
  // only makes sense there. Login and onboarding have their own in-flow
  // cancel affordances.
  const ESCAPE_HATCH_ROUTES = ["/", "/dashboard", "/settings", "/server"];
  const ESCAPE_HATCH_DELAY_MS = 10000;
  // Staged reveal: the spinner shows instantly, the status copy only appears
  // once a load is slow enough for text to be worth reading.
  const STATUS_DELAY_MS = 1000;
  // Fidget copy cycled beneath the headline, mirroring the login page's
  // connecting view.
  const STATUS_PHRASES = [
    "Getting things ready…",
    "Connecting to your BVC Server…",
    "Almost there…",
  ];
  const PHRASE_INTERVAL_MS = 1600;

  class AppPreloaderController {
    constructor(root) {
      this.root = root;
      this.frameEl = root.querySelector("#app-preloader-frame");
      this.headlineEl = root.querySelector("#app-preloader-headline");
      this.statusEl = root.querySelector("#app-preloader-status");
      this.actionsEl = root.querySelector("#app-preloader-actions");
      this.frame = 0;
    }

    isDismissed() {
      return !document.body.contains(this.root);
    }

    scheduleStatus() {
      setTimeout(() => {
        if (this.isDismissed()) return;
        this.headlineEl.hidden = false;
        this.statusEl.hidden = false;

        let phrase = 0;
        const phraseId = setInterval(() => {
          if (this.isDismissed()) {
            clearInterval(phraseId);
            return;
          }
          phrase = (phrase + 1) % STATUS_PHRASES.length;
          this.statusEl.textContent = STATUS_PHRASES[phrase];
        }, PHRASE_INTERVAL_MS);
      }, STATUS_DELAY_MS);
    }

    startMotion() {
      if (window.matchMedia("(prefers-reduced-motion: reduce)").matches) return;
      const frameId = setInterval(() => {
        if (this.isDismissed()) {
          clearInterval(frameId);
          return;
        }
        this.frame = (this.frame + 1) % BRAILLE_FRAMES.length;
        this.frameEl.textContent = BRAILLE_FRAMES[this.frame];
      }, FRAME_INTERVAL_MS);
    }

    armHatch() {
      const path = window.location.pathname.replace(/\/+$/, "") || "/";
      if (!ESCAPE_HATCH_ROUTES.includes(path)) return;

      // Switching servers only makes sense when another server exists, and is
      // circular when the stuck page IS the server picker. The count is
      // mirrored into localStorage by ServerListStore because the Tauri store
      // is unreachable before hydration; unreadable storage counts as zero so
      // only the sign-out escape shows.
      let serverCount = 0;
      try {
        serverCount = parseInt(window.localStorage.getItem("bvc_server_count") ?? "0", 10);
      } catch (_) {}
      if (path === "/server" || !(serverCount > 1)) {
        this.root.querySelector("#app-preloader-action-server").hidden = true;
      }

      this.root
        .querySelector("#app-preloader-keep-waiting")
        .addEventListener("click", () => {
          this.actionsEl.hidden = true;
          this.headlineEl.textContent = "Loading…";
          this.scheduleReveal();
        });

      this.scheduleReveal();
    }

    scheduleReveal() {
      setTimeout(() => {
        if (this.isDismissed()) return;
        this.headlineEl.textContent = "This is taking longer than expected.";
        this.actionsEl.hidden = false;
      }, ESCAPE_HATCH_DELAY_MS);
    }

    start() {
      this.startMotion();
      this.scheduleStatus();
      this.armHatch();
    }
  }

  const root = document.querySelector(".app-preloader");
  if (root) new AppPreloaderController(root).start();
})();
