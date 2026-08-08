import { I18n } from "$lib/i18n";
import { writable, type Readable, type Writable } from "svelte/store";
import { Store } from "@tauri-apps/plugin-store";
import { invoke } from "@tauri-apps/api/core";
import { info, error as logError } from "@tauri-apps/plugin-log";
import { platform } from "@tauri-apps/plugin-os";
import Analytics from "../../analytics";
import type { LinkJavaIdentityResponse } from "../../../bindings/LinkJavaIdentityResponse";
import type { Game } from "../../../bindings/Game";
import { AppStore } from "../../services/AppStore";

export class AccountManager {
    private gamertagStore: Writable<string>;
    public readonly gamertag: Readable<string>;
    private gamerpicStore: Writable<string>;
    public readonly gamerpic: Readable<string>;
    private minecraftUsernameStore: Writable<string | null>;
    public readonly minecraftUsername: Readable<string | null>;
    private isLinkingStore: Writable<boolean>;
    public readonly isLinking: Readable<boolean>;
    private linkErrorStore: Writable<string>;
    public readonly linkError: Readable<string>;
    private isReadyStore: Writable<boolean>;
    public readonly isReady: Readable<boolean>;
    private isDesktopStore: Writable<boolean>;
    public readonly isDesktop: Readable<boolean>;
    private activeGameStore: Writable<Game>;
    public readonly activeGame: Readable<Game>;

    constructor() {
        this.gamertagStore = writable("");
        this.gamertag = { subscribe: this.gamertagStore.subscribe };
        this.gamerpicStore = writable("");
        this.gamerpic = { subscribe: this.gamerpicStore.subscribe };
        this.minecraftUsernameStore = writable(null);
        this.minecraftUsername = { subscribe: this.minecraftUsernameStore.subscribe };
        this.isLinkingStore = writable(false);
        this.isLinking = { subscribe: this.isLinkingStore.subscribe };
        this.linkErrorStore = writable("");
        this.linkError = { subscribe: this.linkErrorStore.subscribe };
        this.isReadyStore = writable(false);
        this.isReady = { subscribe: this.isReadyStore.subscribe };
        this.isDesktopStore = writable(false);
        this.isDesktop = { subscribe: this.isDesktopStore.subscribe };
        this.activeGameStore = writable("minecraft");
        this.activeGame = { subscribe: this.activeGameStore.subscribe };
    }

    async initialize(): Promise<void> {
        try {
            const os = platform();
            this.isDesktopStore.set(os === "windows" || os === "macos" || os === "linux");

            const store = await AppStore.load();
            const currentServer = await store.get<string>("current_server");

            if (!currentServer) return;

            const game = await store.get<string>("active_game");
            this.activeGameStore.set((game === "hytale") ? "hytale" : "minecraft");

            this.gamertagStore.set(await invoke<string>("get_credential", { server: currentServer, key: "gamertag" }).catch(() => ""));
            this.gamerpicStore.set(await invoke<string>("get_credential", { server: currentServer, key: "gamerpic" }).catch(() => ""));

            try {
                const raw = await invoke<string>("get_credential", { server: currentServer, key: "minecraft_username" });
                this.minecraftUsernameStore.set((!raw || raw === "null" || raw === "") ? null : raw);
            } catch {
                this.minecraftUsernameStore.set(null);
            }
        } catch (e) {
            logError(`Failed to load account info: ${e}`);
        }
        this.isReadyStore.set(true);
    }

    async handleLinkJavaIdentity(): Promise<void> {
        this.isLinkingStore.set(true);
        this.linkErrorStore.set("");

        try {
            const store = await AppStore.load();
            const currentServer = await store.get<string>("current_server");

            if (!currentServer) {
                this.linkErrorStore.set(I18n.t("Not connected to a server."));
                this.isLinkingStore.set(false);
                return;
            }

            let gamertag = "";
            this.gamertagStore.update((value) => {
                gamertag = value;
                return value;
            });

            const response = await invoke("link_java_identity", {
                gamertag: gamertag,
            }) as LinkJavaIdentityResponse;

            if (response.minecraft_username) {
                this.minecraftUsernameStore.set(response.minecraft_username);

                await invoke("set_credential", {
                    server: currentServer,
                    key: "minecraft_username",
                    value: response.minecraft_username
                });

                info(`Linked Java identity: ${response.minecraft_username}`);
                Analytics.track("JavaIdentityLinked");
            } else {
                this.linkErrorStore.set(I18n.t("Could not retrieve Java username."));
            }
        } catch (e) {
            logError(`Failed to link Java identity: ${e}`);
            const errorStr = String(e);
            if (errorStr.includes("closed without completing")) {
                this.linkErrorStore.set("");
            } else {
                this.linkErrorStore.set(I18n.t("Failed to link Java identity."));
            }
        }

        this.isLinkingStore.set(false);
    }
}
