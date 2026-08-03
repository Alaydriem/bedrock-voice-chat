import { defineConfig } from "vitest/config";
import { svelte } from "@sveltejs/vite-plugin-svelte";
import { fileURLToPath } from "node:url";

// Runes are not forced: the app build detects them per file, and forcing them here
// would fail on the legacy components that new screens still compose with.
export default defineConfig({
  plugins: [svelte()],
  resolve: {
    // Component tests import kit code the same way app code does.
    alias: {
      $radial: fileURLToPath(new URL("./src/radial", import.meta.url)),
    },
    // Without this Svelte resolves to its server build and mount() throws:
    // happy-dom is a DOM, so the browser entry is the correct one.
    conditions: ["browser"],
  },
  test: {
    environment: "happy-dom",
    setupFiles: ["./src/test/setup.ts"],
    include: ["src/**/*.test.ts"],
    // The kit runs on node --test by design and must not be swept up here.
    exclude: ["src/radial/tests/**", "node_modules/**"],
  },
});
