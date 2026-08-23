import { describe, expect, it } from "vitest";
import { mockInvoke } from "../../tauri";
import { ViewerPermissionsManager } from "../../../js/app/managers/ViewerPermissionsManager";

describe("ViewerPermissionsManager", () => {
    it("reads the permission set the login cached in the keyring", async () => {
        mockInvoke({
            get_credential: () => JSON.stringify({ allowed: ["admin", "audio_upload"] }),
        });

        expect(await new ViewerPermissionsManager().load()).toEqual(["admin", "audio_upload"]);
    });

    // A keyring read that fails must not be read as "everything is permitted".
    it("grants nothing when the cached set cannot be read", async () => {
        mockInvoke({
            get_credential: () => {
                throw new Error("no such credential");
            },
        });

        expect(await new ViewerPermissionsManager().load()).toEqual([]);
    });

    // The point of the refresh: a permission granted after login is invisible in the cache.
    it("prefers what the server says now over the cached set", async () => {
        mockInvoke({
            get_credential: () => JSON.stringify({ allowed: [] }),
            api_introspect: () => ({
                gamertag: "RootAdmin",
                game: "minecraft",
                cert_not_after: 4_000_000_000,
                permissions: ["admin"],
            }),
        });

        expect(await new ViewerPermissionsManager().refresh()).toEqual(["admin"]);
    });

    // An unreachable server must not revoke the pane an operator already had.
    it("falls back to the cached set when the server cannot be reached", async () => {
        mockInvoke({
            get_credential: () => JSON.stringify({ allowed: ["admin"] }),
            api_introspect: () => {
                throw new Error("unreachable");
            },
        });

        expect(await new ViewerPermissionsManager().refresh()).toEqual(["admin"]);
    });

    it("publishes the loaded set on its store", async () => {
        mockInvoke({ get_credential: () => JSON.stringify({ allowed: ["admin"] }) });

        const manager = new ViewerPermissionsManager();
        await manager.load();

        let seen: readonly string[] = [];
        const stop = manager.permissions.subscribe((value) => (seen = value));
        stop();

        expect(seen).toEqual(["admin"]);
    });
});
