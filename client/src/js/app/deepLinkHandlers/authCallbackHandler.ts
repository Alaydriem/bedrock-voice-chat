import { Store } from '@tauri-apps/plugin-store';
import { invoke } from "@tauri-apps/api/core";
import { info, error as logError } from '@tauri-apps/plugin-log';
import Analytics from '../analytics';
import { type LoginResponse } from "../../bindings/LoginResponse";
import { type ServerListEntry } from "../../bindings/ServerListEntry";
import { ServerListStore } from '../services/ServerListStore';
import type { DeepLinkOutcome } from '../deepLinkRouter.ts';

export class AuthCallbackHandler {
    private readonly AUTH_PREFIXES = [
        'bedrock-voice-chat://auth',
        'https://bvc.alaydriem.com/auth',
        'https://www.bedrockvoicechat.com/auth'
    ];
    // Must match Login.LOGIN_ERROR_KEY. Duplicated rather than imported to avoid
    // a Login -> BVCApp -> deepLinkRouter -> authCallbackHandler -> Login cycle.
    private readonly LOGIN_ERROR_KEY = "login_error";
    // Must match DeepLinkRouter.PENDING_KEY. Cleared here, before navigating, so
    // the single-use callback is never re-redeemed on the next page mount.
    private readonly PENDING_KEY = "pending_deep_link";
    private store: Store;

    constructor(store: Store) {
        this.store = store;
    }

    canHandle(url: string): boolean {
        return this.AUTH_PREFIXES.some(prefix => url.startsWith(prefix));
    }

    async handle(url: string): Promise<DeepLinkOutcome> {
        info(`AuthCallbackHandler: Processing auth callback`);

        const parsedUrl = new URL(url);
        const code = parsedUrl.searchParams.get("code");
        const state = parsedUrl.searchParams.get("state");

        if (!code || !state) {
            throw new Error("Auth callback missing required parameters");
        }

        // Redemption has no page dependency: redeem wherever the link arrives and
        // navigate based on the outcome (onboarding on success, /login on
        // retryable failure, /error on access denial). This avoids stranding the
        // callback when the landing page (splash, server list) navigates away
        // before a deferred /login redirect can take effect.
        await this.processAuthCallback(code, state);
        return 'handled';
    }

    private async processAuthCallback(code: string, state: string): Promise<void> {
        const authStateToken = await this.store.get<string>("auth_state_token");
        const authStateEndpoint = await this.store.get<string>("auth_state_endpoint");

        if (state !== authStateToken) {
            logError(`AuthCallbackHandler: Auth state mismatch - Expected: ${authStateToken}, Got: ${state}`);
            await this.failLogin("Authentication failed. Please try again.");
            return;
        }

        if (!authStateEndpoint) {
            logError("AuthCallbackHandler: auth_state_endpoint is undefined");
            await this.failLogin("Login failed. Please check your server URL and try again.");
            return;
        }

        const redirectUri = this.getRedirectUrl();

        try {
            const response = await invoke("server_login", {
                code: code,
                server: authStateEndpoint,
                redirect: redirectUri
            }) as LoginResponse;

            await this.store.set("current_server", authStateEndpoint);
            await this.store.set("current_player", response.gamertag);
            await this.store.set("active_game", "minecraft");

            if (await this.store.has("server_list")) {
                let serverList = await this.store.get("server_list") as ServerListEntry[];
                let hasServer = false;
                serverList.forEach(server => {
                    if (server.server == authStateEndpoint) {
                        hasServer = true;
                        server.game = "minecraft";
                    }
                });

                if (!hasServer) {
                    serverList.push({
                        "server": authStateEndpoint,
                        "player": response.gamertag,
                        "game": "minecraft"
                    });
                }
                await this.store.set("server_list", serverList);
                ServerListStore.mirrorServerCount(serverList);
            } else {
                const serverList: ServerListEntry[] = [{
                    "server": authStateEndpoint,
                    "player": response.gamertag,
                    "game": "minecraft"
                }];
                await this.store.set("server_list", serverList);
                ServerListStore.mirrorServerCount(serverList);
            }

            await this.store.delete("auth_state_token");
            await this.store.delete("auth_state_endpoint");
            await this.store.delete(this.PENDING_KEY);
            await this.store.save();

            Analytics.track("LoginCompleted", { game_type: "minecraft" });
            window.location.href = "/setup";
        } catch (e) {
            logError(`AuthCallbackHandler: Login failed: ${e}`);
            const errorStr = String(e).toLowerCase();
            if (errorStr.includes("403") || errorStr.includes("forbidden") || errorStr.includes("denied") || errorStr.includes("banned") || errorStr.includes("whitelist")) {
                await this.denyLogin();
            } else {
                await this.failLogin("Login failed. Please check your server URL and try again.");
            }
        }
    }

    private getRedirectUrl(): string {
        return "bedrock-voice-chat://auth";
    }

    /**
     * The server rejected the account itself (403: not registered on the server,
     * or banished). This is not recoverable by retrying, so it gets a hard error
     * page rather than an inline login-form warning.
     */
    private async denyLogin(): Promise<void> {
        try {
            await this.store.delete(this.PENDING_KEY);
            await this.store.save();
        } catch (e) {
            logError(`AuthCallbackHandler: Failed to clear pending deep link: ${e}`);
        }
        window.location.href = "/error?code=AUTH02";
    }

    /**
     * Persist a user-facing error, clear the pending callback so its single-use
     * code is not retried, and return to the login page. The login page surfaces
     * the persisted error on mount (consumeAuthError) and live (watchAuthError),
     * so the failure is shown whether or not the page was still mounted.
     */
    private async failLogin(message: string): Promise<void> {
        try {
            await this.store.set(this.LOGIN_ERROR_KEY, message);
            await this.store.delete(this.PENDING_KEY);
            await this.store.save();
        } catch (e) {
            logError(`AuthCallbackHandler: Failed to persist login error: ${e}`);
        }
        // When the login page is already mounted, its watchAuthError listener has
        // consumed the persisted key and rendered the message. Reloading here would
        // wipe that render, and the remount's consumeAuthError would find the key
        // already cleared - the failure would never be shown. Only navigate when
        // the callback landed on some other page.
        const path = window.location.pathname.replace(/\/+$/, "");
        if (path !== "/login") {
            window.location.href = "/login";
        }
    }
}
