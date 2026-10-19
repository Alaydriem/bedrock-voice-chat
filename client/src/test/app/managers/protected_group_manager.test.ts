import { get } from "svelte/store";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { invokeCalls, mockInvoke } from "../../tauri";
import "../../tauri";

// Every handler by event name, so a test can deliver any of the protected-group events and
// see which listeners were released.
const webview = vi.hoisted(() => ({
    listeners: new Map<string, (event: unknown) => void>(),
    released: [] as string[],
}));

vi.mock("@tauri-apps/api/webviewWindow", () => ({
    getCurrentWebviewWindow: () => ({
        listen: async (name: string, handler: (event: unknown) => void) => {
            webview.listeners.set(name, handler);
            return () => webview.released.push(name);
        },
        emit: async () => {},
        label: "main",
    }),
}));

const { ProtectedGroupManager } = await import("../../../js/app/managers/ProtectedGroupManager");

function deliver(name: string, payload: unknown): void {
    const handler = webview.listeners.get(name);
    if (!handler) throw new Error(`no listener for ${name}`);
    handler({ payload });
}

async function started() {
    const manager = new ProtectedGroupManager();
    await manager.startListening();
    return manager;
}

beforeEach(() => {
    webview.listeners.clear();
    webview.released = [];
});

describe("ProtectedGroupManager", () => {
    // The server's channel list is the server's word; a protected group's roster must be the
    // group's own, or a hidden listener is exactly as hidden as before.
    it("roster_for_protected_channel_comes_from_protected_group_events", async () => {
        const manager = await started();

        deliver("protected-group:roster", { members: ["minecraft:Alice", "minecraft:Bob"] });

        expect(get(manager.roster)).toEqual(["minecraft:Alice", "minecraft:Bob"]);
    });

    it("joined_and_epoch_events_set_the_safety_code_and_mark_the_group_active", async () => {
        const manager = await started();

        deliver("protected-group:joined", { epoch: 3, safety_code: "11111 22222 33333 44444 55555 66666" });
        expect(get(manager.active)).toBe(true);
        expect(get(manager.epoch)).toBe(3);

        deliver("protected-group:epoch", { epoch: 4, safety_code: "99999 88888 77777 66666 55555 44444" });
        expect(get(manager.epoch)).toBe(4);
        expect(get(manager.safetyCode)).toBe("99999 88888 77777 66666 55555 44444");
    });

    it("lost_reason_maps_to_exact_copy", () => {
        expect(ProtectedGroupManager.lostMessage("wrong_password")).toBe("Wrong password.");
        expect(ProtectedGroupManager.lostMessage("no_admitter")).toBe("No member is online to admit you.");
        expect(ProtectedGroupManager.lostMessage("admitter_unreachable")).toBe("Could not reach a group member.");
        expect(ProtectedGroupManager.lostMessage("socket_closed")).toBe(
            "Connection to the group was lost. Rejoin to continue.",
        );
        expect(ProtectedGroupManager.lostMessage("protocol_failure")).toBe(
            "Connection to the group was lost. Rejoin to continue.",
        );
    });

    it("a_lost_event_clears_the_group_and_keeps_the_reason", async () => {
        const manager = await started();
        deliver("protected-group:joined", { epoch: 1, safety_code: "x" });
        deliver("protected-group:roster", { members: ["minecraft:Alice"] });

        deliver("protected-group:lost", { reason: "socket_closed" });

        expect(get(manager.active)).toBe(false);
        expect(get(manager.roster)).toEqual([]);
        expect(get(manager.safetyCode)).toBeNull();
        expect(get(manager.lostReason)).toBe("socket_closed");
    });

    it("protected_join_invokes_group_join_protected_with_password", async () => {
        mockInvoke({ group_join_protected: () => ({ in_group: true, id: "AB12CD34", name: null }) });
        const manager = await started();

        await manager.join("AB12CD34", "correct horse");

        const call = invokeCalls().find((c) => c.cmd === "group_join_protected");
        expect(call?.args).toEqual({ id: "AB12CD34", password: "correct horse" });
    });

    // Leaving wipes the keys first and lifts protection only after the channel is left; stopping
    // alone must never let audio go out unsealed.
    it("stop_keeps_protection_until_release", async () => {
        mockInvoke({ group_stop_protected: () => null, group_release_protected: () => null });
        const manager = await started();

        await manager.stop();
        expect(invokeCalls().map((c) => c.cmd)).not.toContain("group_release_protected");

        await manager.release();
        expect(invokeCalls().map((c) => c.cmd)).toContain("group_release_protected");
    });

    it("cleanup_unlistens_all_protected_group_events", async () => {
        const manager = await started();

        manager.cleanup();

        expect(webview.released.sort()).toEqual([
            "protected-group:epoch",
            "protected-group:joined",
            "protected-group:lost",
            "protected-group:roster",
        ]);
    });
});
