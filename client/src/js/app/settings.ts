import { Store } from '@tauri-apps/plugin-store';
import BVCApp from "./BVCApp";

declare global {
  interface Window {
    App: any;
  }
}

export default class Dashboard extends BVCApp {
    private store: Store | undefined;

    async initialize() {
        // Application-level initialization can go here
        // Component mounting is now handled by Svelte template
    }
}