import { defineConfig } from "vitest/config";
import { svelte } from "@sveltejs/vite-plugin-svelte";
import { fileURLToPath } from "node:url";

// Runes are not forced, matching the app build, which detects them per file. Every
// component is on runes now, so this is a default rather than an accommodation.
export default defineConfig({
  plugins: [svelte()],
  resolve: {
    // Component tests import kit code the same way app code does.
    alias: {
      $radial: fileURLToPath(new URL("./src/radial", import.meta.url)),
      // SvelteKit resolves $lib through its own plugin, which is absent here for the
      // same reason $app is below. Components importing I18n are unloadable without it.
      $lib: fileURLToPath(new URL("./src/lib", import.meta.url)),
      // SvelteKit's own plugin is not in this pipeline, so its ambient modules
      // have to come from somewhere. Route components are unimportable without
      // them, coverage included.
      "$app/navigation": fileURLToPath(new URL("./src/test/kit/navigation.ts", import.meta.url)),
      "$app/state": fileURLToPath(new URL("./src/test/kit/state.ts", import.meta.url)),
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
    coverage: {
      provider: "v8",
      // `text` for the CI log, `lcov` for Codecov.
      reporter: ["text-summary", "lcov"],
      reportsDirectory: "./coverage",
      // Reported whether or not a test touched them. Without this a module with
      // no test at all is absent from the report rather than shown at zero,
      // which reads as full coverage of a smaller codebase.
      all: true,
      include: ["src/**/*.{ts,svelte}"],
      exclude: [
        // Generated from the Rust types in common/. The generator is what is
        // under test, not its output.
        "src/js/bindings/**",
        "src/test/**",
        "src/radial/tests/**",
        // The reference gallery: design sources built to dist-radial, not
        // shipped in the app.
        "src/radial/examples/**",
        "src/**/*.d.ts",
      ],
    },
  },
});
