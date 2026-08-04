import { openUrl } from "@tauri-apps/plugin-opener";

/**
 * The places we send someone who is stuck.
 *
 * Both failure surfaces need these — the login flow's connect-error view and the error
 * route — and they were private to `Login`, which is not somewhere a terminal error screen
 * should have to instantiate to read three strings.
 *
 * Always the system browser, never the app window: the WebView has no chrome, so a route
 * navigated to GitHub is a dead end with no way back.
 */
export default class HelpLinks {
  static readonly WIKI = "https://github.com/alaydriem/bedrock-voice-chat";
  static readonly DISCORD = "https://discord.gg/WGXy5kBP9E";
  static readonly PRIVACY =
    "https://raw.githubusercontent.com/Alaydriem/bedrock-voice-chat/refs/heads/master/PRIVACY_STATEMENT.md";

  static openWiki(): Promise<void> {
    return openUrl(HelpLinks.WIKI);
  }

  static openDiscord(): Promise<void> {
    return openUrl(HelpLinks.DISCORD);
  }

  static openPrivacyNotice(): Promise<void> {
    return openUrl(HelpLinks.PRIVACY);
  }
}
