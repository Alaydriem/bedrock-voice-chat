import Main from "./main";

import { platform } from "@tauri-apps/plugin-os";
import { check, Update } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";
import { warn, debug, trace, info, error } from "@tauri-apps/plugin-log";
import { match, Pattern } from "ts-pattern";

declare global {
  interface Window {
    App: any;
  }
}

export default class Splash extends Main {
  constructor() {
    super();
    this.update();
  }

  update() {
    if (this.isMobile()) {
      // If this is a mobile app, then redirect to the server page
      window.location.href = "/server";
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

  isMobile() {
    const currentPlatform = platform();
    return currentPlatform === "ios" || currentPlatform === "android";
  }
}
