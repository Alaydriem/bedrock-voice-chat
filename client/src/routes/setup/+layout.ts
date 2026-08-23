// Tauri has no Node.js server to do proper SSR, so the app is prerendered as SSG.
// See: https://v2.tauri.app/start/frontend/sveltekit/
export const prerender = true;
export const ssr = false;
