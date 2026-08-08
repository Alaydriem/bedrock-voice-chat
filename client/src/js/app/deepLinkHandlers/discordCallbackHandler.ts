import { I18n } from "$lib/i18n";
import { invoke } from "@tauri-apps/api/core";
import { info, error as logError } from "@tauri-apps/plugin-log";
import type { DeepLinkOutcome } from "../deepLinkRouter.ts";

export class DiscordCallbackHandler {
    // The https Universal/App Link registered with Discord, plus the custom-scheme
    // form used by the iOS fragment-trampoline fallback.
    private readonly PREFIXES = [
        "https://www.bedrockvoicechat.com/discord/callback",
        "bedrock-voice-chat://discord-callback"
    ];

    canHandle(url: string): boolean {
        return this.PREFIXES.some(prefix => url.startsWith(prefix));
    }

    async handle(url: string): Promise<DeepLinkOutcome> {
        info("DiscordCallbackHandler: processing callback");

        // The implicit-grant token is in the URL fragment for the Universal/App
        // Link; the trampoline fallback carries it in the query string instead.
        const hashIndex = url.indexOf("#");
        const queryIndex = url.indexOf("?");
        let payload = "";
        if (hashIndex >= 0) {
            payload = url.slice(hashIndex + 1);
        } else if (queryIndex >= 0) {
            payload = url.slice(queryIndex + 1);
        }

        if (!payload) {
            throw new Error(I18n.t("Discord callback missing token payload"));
        }

        try {
            await invoke("discord_complete_link", { fragment: payload });
            window.dispatchEvent(new CustomEvent("discord-link-updated"));
        } catch (e) {
            logError(`DiscordCallbackHandler: complete failed: ${e}`);
            throw e;
        }
        return "handled";
    }
}
