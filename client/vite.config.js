import { defineConfig } from "vite";
import { sveltekit } from "@sveltejs/kit/vite";
import tailwindcss from "@tailwindcss/vite";
import { sentryVitePlugin } from "@sentry/vite-plugin";

// @ts-expect-error process is a nodejs global
const host = process.env.TAURI_DEV_HOST;

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
