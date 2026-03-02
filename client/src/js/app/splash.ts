import { invoke } from "@tauri-apps/api/core";
import { info, error } from "@tauri-apps/plugin-log";
import PlatformDetector from "./utils/PlatformDetector.ts";
import BVCApp from "./BVCApp.ts";

declare global {
  interface Window {
    App: any;
  }
}

export default class Splash extends BVCApp {
  private platformDetector: PlatformDetector | undefined;
  constructor() {
    super();
    this.platformDetector = new PlatformDetector();
  }

  async initialize() {
    await this.initializeDeepLinks();
    await this.update();
  }

  async update() {
    const isMobile = await this.platformDetector?.checkMobile();
    if (isMobile) {
      window.location.href = "/server";
      return;
    }

    try {
      // Rust-side update check with dynamic endpoint selection.
      // If an update is found, Rust downloads, installs, and restarts the app.
      // If no update, returns null and we navigate to /server.
      await invoke<string | null>("check_for_updates");
      info("No updates available.");
      window.location.href = "/server";
    } catch (e) {
      error("Update check failed: " + e);
      window.location.href = "/server";
    }
  }
}
