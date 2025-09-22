// Tauri doesn't have a Node.js server to do proper SSR
// so we will use adapter-static to prerender the app (SSG)
// See: https://v2.tauri.app/start/frontend/sveltekit/ for more info
import adapter from "@sveltejs/adapter-static";
import { vitePreprocess } from "@sveltejs/vite-plugin-svelte";

/** @type {import('@sveltejs/kit').Config} */
const config = {
  preprocess: vitePreprocess(),
  onwarn: (warning, handler) => {
    // Suppress warnings for Alpine.js directives and attributes
    const alpineDirectives = [
      'x-data', 'x-show', 'x-hide', 'x-if', 'x-for', 'x-transition',
      'x-ref', 'x-cloak', 'x-ignore', 'x-effect', 'x-html', 'x-text',
      'x-on:', '@click', '@input', '@change', '@submit', '@keydown',
      'x-bind:', ':class', ':style', ':href', ':src', ':disabled',
      'x-model', 'x-init', 'x-tooltip', 'data-x-'
    ];
    
    if (warning.code === 'a11y-unknown-attribute' || 
        warning.code === 'attribute-invalid-name' ||
        warning.code === 'unknown-attribute') {
      const attributeName = warning.message?.match(/'([^']+)'/)?.[1] || '';
      if (alpineDirectives.some(directive => attributeName.startsWith(directive))) {
        return; // Suppress this warning
      }
    }
    
    // Let other warnings through
    handler(warning);
  },
  kit: {
    adapter: adapter(),
  },
};

export default config;
