import { invoke } from "@tauri-apps/api/core";
import { error as logError } from "@tauri-apps/plugin-log";
import { writable, type Readable, type Writable } from "svelte/store";
import type { IntrospectResponse } from "../../bindings/IntrospectResponse";
import type { Permission } from "../../bindings/Permission";
import type { ViewerIdentity } from "./ViewerIdentity";
import { AppStore } from "../services/AppStore";

/**
 * What the signed-in player is allowed to do on the server they are signed in to.
 *
 * The login already writes the evaluated set into the keyring, so the cached read is
 * instant and works offline; that is what gates a menu item without a round trip. The
 * refresh exists because the cache is only as new as the last login, and a permission
 * granted while the app is open would otherwise need a sign-out to appear.
 */
export class ViewerPermissionsManager {
    private permissionsStore: Writable<readonly Permission[]>;
    public readonly permissions: Readable<readonly Permission[]>;
    private identityStore: Writable<ViewerIdentity | null>;
    /**
     * Who the viewer is, which is what tells a roster row apart from its operator.
     *
     * Null until something answers. A caller must read that as "not known yet" rather than
     * "not this row": an action refused for self has to stay hidden, and guessing wrong in
     * that direction offers a button the server will only reject.
     */
    public readonly identity: Readable<ViewerIdentity | null>;

    constructor() {
        this.permissionsStore = writable([]);
        this.permissions = { subscribe: this.permissionsStore.subscribe };
        this.identityStore = writable(null);
        this.identity = { subscribe: this.identityStore.subscribe };
    }

    /** The set the login cached, and the identity beside it. No network. */
    async load(): Promise<readonly Permission[]> {
        const cached = await this.cached();
        this.permissionsStore.set(cached);
        await this.loadIdentity();
        return cached;
    }

    /**
     * The set the server reports now, which the command also writes back to the cache.
     *
     * A failure falls back to the cache rather than to nothing: an unreachable server is
     * not a revocation, and treating it as one would take a pane away from an operator
     * mid-task.
     */
    async refresh(): Promise<readonly Permission[]> {
        try {
            const response = await invoke<IntrospectResponse>("api_introspect", { server: null });
            const live = response.permissions ?? [];
            this.permissionsStore.set(live);
            // The server reports the identity its certificate proved, which is the one every
            // admin route compares against. Authoritative over the cached gamertag.
            if (response.gamertag) {
                this.identityStore.set({ gamertag: response.gamertag, game: response.game });
            }
            return live;
        } catch (e) {
            logError(`Failed to refresh permissions: ${e}`);
            return this.load();
        }
    }

    private async cached(): Promise<readonly Permission[]> {
        try {
            const store = await AppStore.load();
            const server = await store.get<string>("current_server");
            if (!server) return [];
            const raw = await invoke<string>("get_credential", {
                server,
                key: "server_permissions",
            });
            const parsed = raw ? (JSON.parse(raw) as { allowed?: Permission[] }) : null;
            return parsed?.allowed ?? [];
        } catch {
            // A failed read grants nothing.
            return [];
        }
    }

    /**
     * The identity from the keyring, for the offline path.
     *
     * The login wrote both, so a self-only action stays correctly hidden on a launch that
     * never reaches the server.
     */
    private async loadIdentity(): Promise<void> {
        try {
            const store = await AppStore.load();
            const server = await store.get<string>("current_server");
            if (!server) return;
            const gamertag = await invoke<string>("get_credential", {
                server,
                key: "gamertag",
            });
            if (!gamertag) return;
            const game = await store.get<string>("active_game");
            this.identityStore.set({ gamertag, game: game === "hytale" ? "hytale" : "minecraft" });
        } catch {
            // Unknown, which is what the store already says.
        }
    }

    static has(permissions: readonly Permission[], permission: Permission): boolean {
        return permissions.includes(permission);
    }
}
