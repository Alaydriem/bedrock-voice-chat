import { Store } from '@tauri-apps/plugin-store';
import { invoke } from "@tauri-apps/api/core";
import { info, error as logError } from '@tauri-apps/plugin-log';
import Analytics from '../analytics';
import { type LoginResponse } from "../../bindings/LoginResponse";
import { type ServerListEntry } from "../../bindings/ServerListEntry";
import type { DeepLinkOutcome } from '../deepLinkRouter.ts';

export class AuthCallbackHandler {
    private readonly AUTH_PREFIXES = [
        'bedrock-voice-chat://auth',
        'https://bvc.alaydriem.com/auth'
    ];
    // Must match Login.LOGIN_ERROR_KEY. Duplicated rather than imported to avoid
    // a Login -> BVCApp -> deepLinkRouter -> authCallbackHandler -> Login cycle.
    private readonly LOGIN_ERROR_KEY = "login_error";
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

        const currentPath = window.location.pathname;

        if (currentPath === "/login") {
            await this.processAuthCallback(url, code, state);
            return 'handled';
        }

        // The webview isn't on the login page (e.g. the deep link arrived while
        // elsewhere in the app). Navigate there and keep the pending entry so
        // the login page picks the callback up after it loads.
        window.location.href = "/login";
        return 'deferred';
    }

    private async processAuthCallback(url: string, code: string, state: string): Promise<void> {
        const authStateToken = await this.store.get<string>("auth_state_token");
        const authStateEndpoint = await this.store.get<string>("auth_state_endpoint");

        if (state !== authStateToken) {
            const errorMsg = `Auth State Mismatch - Expected: ${authStateToken}, Got: ${state}`;
            await this.showLoginError("Authentication failed. Please try again.");
            throw new Error(errorMsg);
        }

        const redirectUri = this.getRedirectUrl();

        try {
            const response = await invoke("server_login", {
                code: code,
                server: authStateEndpoint,
                redirect: redirectUri
            }) as LoginResponse;

            if (authStateEndpoint) {
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
                } else {
                    let serverList = [];
                    serverList.push({
                        "server": authStateEndpoint,
                        "player": response.gamertag,
                        "game": "minecraft"
                    });
                    await this.store.set("server_list", serverList);
                }

                await this.store.delete("auth_state_token");
                await this.store.delete("auth_state_endpoint");
                await this.store.save();

                Analytics.track("LoginCompleted", { game_type: "minecraft" });
                window.location.href = "/onboarding/welcome";
            } else {
                throw new Error("authStateEndpoint is undefined");
            }
        } catch (e) {
            logError(`AuthCallbackHandler: Login failed: ${e}`);
            const errorStr = String(e).toLowerCase();
            if (errorStr.includes("403") || errorStr.includes("forbidden") || errorStr.includes("permission") || errorStr.includes("banned") || errorStr.includes("whitelist")) {
                await this.showLoginError("Access denied. Check with your server operator if you have permissions.");
            } else {
                await this.showLoginError("Login failed. Please check your server URL and try again.");
            }
            throw e;
        }
    }

    private getRedirectUrl(): string {
        return "bedrock-voice-chat://auth";
    }

    /**
     * Persist a user-facing error so the login page can surface it. The page
     * observes this store key (live and on mount), so the failure is shown as a
     * warning whether or not the page is still mounted from the original attempt.
     */
    private async showLoginError(message: string): Promise<void> {
        try {
            await this.store.set(this.LOGIN_ERROR_KEY, message);
            await this.store.save();
        } catch (e) {
            logError(`AuthCallbackHandler: Failed to persist login error: ${e}`);
        }
    }
}
