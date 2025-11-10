import { Store } from '@tauri-apps/plugin-store';
import { invoke } from "@tauri-apps/api/core";
import { info, error as logError } from '@tauri-apps/plugin-log';
import { platform } from '@tauri-apps/plugin-os';
import Keyring from '../keyring.ts';
import { type LoginResponse } from "../../bindings/LoginResponse";

export class AuthCallbackHandler {
    private readonly AUTH_PREFIXES = [
        'bedrock-voice-chat://auth',
        'https://bvc.alaydriem.com/auth'
    ];
    private store: Store;

    constructor(store: Store) {
        this.store = store;
    }

    canHandle(url: string): boolean {
        return this.AUTH_PREFIXES.some(prefix => url.startsWith(prefix));
    }

    async handle(url: string): Promise<void> {
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
        } else {
            window.location.href = "/login";
        }
    }

    private async processAuthCallback(url: string, code: string, state: string): Promise<void> {
        const authStateToken = await this.store.get<string>("auth_state_token");
        const authStateEndpoint = await this.store.get<string>("auth_state_endpoint");

        if (state !== authStateToken) {
            const errorMsg = `Auth State Mismatch - Expected: ${authStateToken}, Got: ${state}`;
            this.showLoginError();
            throw new Error(errorMsg);
        }

        const redirectUri = this.getRedirectUrl();

        try {
            const response = await invoke("server_login", {
                code: code,
                server: authStateEndpoint,
                redirect: redirectUri
            }) as LoginResponse;

            const keyring = await Keyring.new("servers");
            if (authStateEndpoint) {
                keyring.setServer(authStateEndpoint);

                Object.keys(response).forEach(async key => {
                    const value = response[key as keyof LoginResponse];
                    if (typeof value === "string" || value instanceof Uint8Array) {
                        await keyring.insert(key, value);
                    } else {
                        await keyring.insert(key, JSON.stringify(value));
                    }
                });

                await this.store.set("current_server", authStateEndpoint);
                await this.store.set("current_player", response.gamertag);

                if (await this.store.has("server_list")) {
                    let serverList = await this.store.get("server_list") as Array<{ server: string, player: string }>;
                    let hasServer = false;
                    serverList.forEach(server => {
                        if (server.server == authStateEndpoint) {
                            hasServer = true;
                        }
                    });

                    if (!hasServer) {
                        serverList.push({
                            "server": authStateEndpoint,
                            "player": response.gamertag
                        });
                        await this.store.set("server_list", serverList);
                    }
                } else {
                    let serverList = [];
                    serverList.push({
                        "server": authStateEndpoint,
                        "player": response.gamertag
                    });
                    await this.store.set("server_list", serverList);
                }

                await this.store.delete("auth_state_token");
                await this.store.delete("auth_state_endpoint");
                await this.store.save();

                window.location.href = "/onboarding/welcome";
            } else {
                throw new Error("authStateEndpoint is undefined");
            }
        } catch (e) {
            logError(`AuthCallbackHandler: Login failed: ${e}`);
            this.showLoginError();
            throw e;
        }
    }

    private getRedirectUrl(): string {
        switch (platform()) {
            case "windows": return "bedrock-voice-chat://auth";
            case "android": return "bedrock-voice-chat://auth";
            case "ios": return "msauth.com.alaydriem.bvc.client://auth";
            default: throw new Error("Unsupported platform");
        }
    }

    private showLoginError(): void {
        const form = document.querySelector("#login-form");
        const serverUrl = form?.querySelector("#bvc-server-input");
        const errorMessage = form?.querySelector("#bvc-server-input-error-message");

        serverUrl?.classList.add("border-error");
        errorMessage?.classList.remove("invisible");
    }
}
