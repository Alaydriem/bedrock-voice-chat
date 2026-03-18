import { invoke } from "@tauri-apps/api/core";
import { info, warn } from "@tauri-apps/plugin-log";
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
      const version = await invoke<string | null>("check_for_updates");
      if (version) {
        info(`Update available: v${version}`);
        window.location.href = `/error?code=UPD01&version=${encodeURIComponent(version)}`;
      } else {
        info("No updates available.");
        window.location.href = "/server";
      }
    } catch (e) {
      warn("Update check failed: " + e);
      window.location.href = "/server";
    }
  }
}
