import { SettingsCatalogue } from "../../../../js/app/settings/SettingsCatalogue";

// Tauri doesn't have a Node.js server to do proper SSR
// so we will use adapter-static to prerender the app (SSG)
// See: https://v2.tauri.app/start/frontend/sveltekit/ for more info
export const prerender = true;
export const ssr = false;

// adapter-static cannot discover a dynamic route on its own, and an undiscovered pane
// is a deep link that works in dev and 404s in the packaged app. Every pane is listed,
// including the desktop-only ones: which of them a build shows is decided at runtime,
// and the same bundle serves both.
export function entries(): { pane: string }[] {
    return SettingsCatalogue.all.map((pane) => ({ pane: pane.id }));
}
