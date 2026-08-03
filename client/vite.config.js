import { defineConfig } from "vite";
import { sveltekit } from "@sveltejs/kit/vite";
import tailwindcss from "@tailwindcss/vite";
import { sentryVitePlugin } from "@sentry/vite-plugin";
import { createHash } from "node:crypto";
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";

const host = process.env.TAURI_DEV_HOST;

const PRELOADER_FILES = ["app-preloader.js", "app-preloader.css"];

// The boot preloader's two files are plain static assets: nothing hashes their
// filenames, so a webview serves a stale copy across reloads and app restarts unless
// the URL itself changes. Their combined content hash becomes the ?v= on both tags in
// app.html, so the URL moves exactly when the files do and never because an unrelated
// rebuild happened.
//
// Read from disk rather than tracked through the bundler because these deliberately are
// not bundle inputs — app-preloader.js is built by vite.preloader.config.ts, which `dev`
// and `build` both run first, and app-preloader.css is hand-written.
function preloaderVersion() {
  const hash = createHash("sha256");
  for (const name of PRELOADER_FILES) {
    try {
      hash.update(readFileSync(fileURLToPath(new URL(`./static/preloader/${name}`, import.meta.url))));
    } catch {
      // Absent until the first `yarn preloader:build`. Hashing what exists still yields
      // a value that changes once the file lands.
    }
  }
  return hash.digest("hex").slice(0, 12);
}

// Patches webaudio-controls `const` reassignment bug that Rolldown rejects
function patchWebaudioControls() {
  return {
    name: "patch-webaudio-controls",
    transform(code, id) {
      if (id.includes("webaudio-controls")) {
        return { code: code.replaceAll("const delta = this.step", "let delta = this.step") };
      }
    },
  };
}

// Set at module scope, not from a plugin hook: SvelteKit reads the environment in a
// `config` hook ordered 'pre', so a plugin of ours would always assign this after
// app.html had already been substituted.
process.env.PUBLIC_PRELOADER_VERSION = preloaderVersion();

// https://vitejs.dev/config/
export default defineConfig(async () => ({
  // Expose SENTRY_DSN to the frontend bundle without requiring a VITE_ prefix
  envPrefix: ["VITE_", "SENTRY_"],

  plugins: [
    patchWebaudioControls(),
    tailwindcss(),
    sveltekit(),
    sentryVitePlugin({
      org: process.env.SENTRY_ORG,
      project: process.env.SENTRY_PROJECT,
      authToken: process.env.SENTRY_AUTH_TOKEN,
      disable: !process.env.SENTRY_AUTH_TOKEN,
    }),
  ],

  build: {
    // Required for Sentry source map uploads to produce meaningful stack traces
    sourcemap: true,
    // Android 11 WebView is Chrome ~87-90. Target Chrome 87 for safety.
    target: ['es2020', 'chrome87'],
  },

  // SvelteKit route nodes are dynamically imported, so Vite's dep scanner
  // never crawls into +page.svelte's transitive imports at startup. The heavy
  // CJS libraries pulled in by js/app/app.ts are therefore discovered only on
  // the first navigation to "/", triggering a second optimization pass whose
  // "optimized dependencies changed. reloading" races the in-flight route node
  // import and serves it malformed (SyntaxError). Pre-bundling them in the
  // first pass eliminates the second pass and the reload race.
  optimizeDeps: {
    include: [
      "@sentry/browser",
      "@tauri-apps/api/core",
      "@tauri-apps/api/event",
      "@tauri-apps/plugin-log",
      "@tauri-apps/plugin-os",
      "@tauri-apps/plugin-store",
      "alpinejs",
      "@alpinejs/collapse",
      "@alpinejs/intersect",
      "@alpinejs/persist",
      "@caneara/iodine",
      "@popperjs/core",
      "apexcharts",
      "cleave.js/dist/cleave.min",
      "dayjs",
      "filepond",
      "filepond-plugin-image-preview",
      "flatpickr",
      "gridjs",
      "quill",
      "simplebar",
      "sortablejs",
      "swiper/bundle",
      "tippy.js",
      "toastify-js",
      "tom-select/dist/js/tom-select.complete.min",
    ],
  },

  // Vite options tailored for Tauri development and only applied in `tauri dev` or `tauri build`
  //
  // 1. prevent vite from obscuring rust errors
  clearScreen: false,
  // 2. tauri expects a fixed port, fail if that port is not available
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host
      ? {
          protocol: "ws",
          host,
          port: 1421,
        }
      : undefined,
    watch: {
      // 3. tell vite to ignore watching `src-tauri`
      ignored: ["**/src-tauri/**"],
    },
  },
}));
