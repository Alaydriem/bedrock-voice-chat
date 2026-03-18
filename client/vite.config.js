import { defineConfig } from "vite";
import { sveltekit } from "@sveltejs/kit/vite";
import tailwindcss from "@tailwindcss/vite";
import { sentryVitePlugin } from "@sentry/vite-plugin";

// @ts-expect-error process is a nodejs global
const host = process.env.TAURI_DEV_HOST;

// https://vitejs.dev/config/
export default defineConfig(async () => ({
  // Expose SENTRY_DSN to the frontend bundle without requiring a VITE_ prefix
  envPrefix: ["VITE_", "SENTRY_"],

  plugins: [
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

  esbuild: {
    supported: {
      'top-level-await': true //browsers can handle top-level-await features
    },
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
