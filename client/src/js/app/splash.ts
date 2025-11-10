import { check, Update } from "@tauri-apps/plugin-updater";
import { info, error } from "@tauri-apps/plugin-log";
import { match, Pattern } from "ts-pattern";
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
      // If this is a mobile app, then redirect to the server page
      window.location.href = "/server";
      return;
    } else {
      // Otherwise, perfrom a self update check
      const update = async () => {
        return await check({
          timeout: 5,
        });
      };

      update()
        .then((result) => {
          match(result)
            .with(Pattern.not(null), (update) => {
              update.downloadAndInstall().then((event) => {
                // https://v2.tauri.app/plugin/updater/#checking-for-updates
                // match the event, download it, then relaunch the app
              });
            })
            .with(null, () => {
              info("No updates returned from server.");
              window.location.href = "/server";
            })
            .exhaustive();
        })
        .catch((e) => {
          error("Update check failed: " + e);
          window.location.href = "/server";
        });
    }
  }
}
