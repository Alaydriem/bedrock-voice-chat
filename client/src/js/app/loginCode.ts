import { info, error as logError } from '@tauri-apps/plugin-log';
import { invoke } from '@tauri-apps/api/core';
import BVCApp from './BVCApp.ts';
import Keyring from './keyring.ts';
import type { LoginResponse } from '../bindings/LoginResponse';
import type { Game } from '../bindings/Game';

export default class LoginCode extends BVCApp {
    private serverUrl = "";

    async initialize() {
        const params = new URLSearchParams(window.location.search);
        this.serverUrl = params.get("server") || "";

        const serverDisplay = document.querySelector("#code-server-display") as HTMLInputElement;
        if (serverDisplay) {
            serverDisplay.value = this.serverUrl;
        }
    }

    async submit(event: Event) {
        event.preventDefault();

        const gamertagInput = document.querySelector("#code-gamertag-input") as HTMLInputElement;
        const codeInput = document.querySelector("#code-input") as HTMLInputElement;
        const gameSelect = document.querySelector("#code-game-select") as HTMLSelectElement;
        const errorEl = document.querySelector("#code-error-message");
        const submitBtn = document.querySelector("#code-submit-btn") as HTMLButtonElement;

        errorEl?.classList.add("invisible");

        const gamertag = gamertagInput?.value.trim() || "";
        const code = codeInput?.value.trim().toUpperCase() || "";
        const game = (gameSelect?.value || "minecraft") as Game;

        if (!gamertag || !code) {
            this.showError(errorEl, "Please enter both your gamertag and code.");
            return;
        }

        if (submitBtn) {
            submitBtn.disabled = true;
        }

        try {
            info(`Attempting code login to ${this.serverUrl}`);

            const response = await invoke<LoginResponse>("code_login", {
                server: this.serverUrl,
                gamertag,
                code,
            });

            const [store, keyring] = await Promise.all([
                this.getStore(),
                Keyring.new("servers"),
            ]);
            await keyring.setServer(this.serverUrl);

            const inserts = Object.keys(response).flatMap(key => {
                const value = response[key as keyof LoginResponse];
                if (value === null || value === undefined) {
                    return [];
                }
                const serialized = (typeof value === "string" || value instanceof Uint8Array)
                    ? value
                    : JSON.stringify(value);
                return [keyring.insert(key, serialized)];
            });

            const [rawServerList] = await Promise.all([
                store.get("server_list") as Promise<Array<{ server: string, player: string, game?: string }> | null>,
                store.set("current_server", this.serverUrl),
                store.set("current_player", response.gamertag),
                store.set("active_game", game),
                ...inserts,
            ]);

            const serverList = rawServerList || [];
            const existing = serverList.find(s => s.server === this.serverUrl);
            if (existing) {
                existing.game = game;
            } else {
                serverList.push({ server: this.serverUrl, player: response.gamertag, game });
            }
            await store.set("server_list", serverList);
            await store.save();

            info("Code login successful, navigating to onboarding");
            window.location.href = "/onboarding/welcome";
        } catch (e) {
            logError(`Code login failed: ${String(e)}`);
            this.showError(errorEl, "Login failed. Check your code and gamertag.");
            if (submitBtn) {
                submitBtn.disabled = false;
            }
        }
    }

    private showError(errorEl: Element | null, message: string) {
        if (errorEl instanceof HTMLElement) {
            errorEl.innerText = message;
            errorEl.classList.remove("invisible");
        }
    }
}
