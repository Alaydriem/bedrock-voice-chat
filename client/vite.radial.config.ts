import { defineConfig } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";
import { vitePreprocess } from "@sveltejs/vite-plugin-svelte";
import tailwindcss from "@tailwindcss/vite";
import { fileURLToPath } from "node:url";
import { readdirSync } from "node:fs";

const radialDir = fileURLToPath(new URL("./src/radial", import.meta.url));
const examplesDir = fileURLToPath(new URL("./src/radial/examples", import.meta.url));

// Every .html in examples/ is a build entry, so adding a page needs no config edit.
const pages = Object.fromEntries(
  readdirSync(examplesDir)
    .filter((f) => f.endsWith(".html"))
    .map((f) => [f.replace(/\.html$/, ""), `${examplesDir}/${f}`]),
);

// The Radial reference lives beside the app but is not part of it: no SvelteKit,
// no Tauri, no Sentry. It builds and serves on its own so the kit can be reviewed
// without a Tauri toolchain present.
export default defineConfig({
  root: "src/radial/examples",

  resolve: {
    alias: { $radial: radialDir },
  },

  plugins: [
    tailwindcss(),
    svelte({
      configFile: false,
      preprocess: vitePreprocess(),
      compilerOptions: { runes: true },
    }),
  ],

  server: {
    port: 1430,
    strictPort: false,
  },

  build: {
    outDir: fileURLToPath(new URL("./dist-radial", import.meta.url)),
    emptyOutDir: true,
    target: ["es2020", "chrome87"],
    rollupOptions: { input: pages },
  },
});
