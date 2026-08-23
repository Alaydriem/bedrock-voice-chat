// Tauri doesn't have a Node.js server to do proper SSR
// so we will use adapter-static to prerender the app (SSG)
// See: https://v2.tauri.app/start/frontend/sveltekit/ for more info
import adapter from "@sveltejs/adapter-static";
import { vitePreprocess } from "@sveltejs/vite-plugin-svelte";

/** @type {import('@sveltejs/kit').Config} */
const config = {
  preprocess: vitePreprocess(),
  kit: {
    adapter: adapter(),
    alias: {
      // The Radial design system. Kit turns this into a tsconfig path too, so
      // `import { Mark } from "$radial/..."` resolves for svelte-check as well as
      // for the bundler. The standalone reference pages get the same alias from
      // vite.radial.config.ts.
      $radial: "src/radial",
    },
  },
};

export default config;
