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
        expect(ProtectedGroupManager.lostMessage("no_admitter")).toBe("No member is online");
        expect(ProtectedGroupManager.lostMessage("expired")).toBe("No member answered");
        expect(ProtectedGroupManager.lostMessage("mismatch")).toBe("The codes did not match");
        expect(ProtectedGroupManager.lostMessage("socket_closed")).toBe(
            "Connection to the group was lost. Rejoin to continue.",
        );
        expect(ProtectedGroupManager.lostMessage("protocol_failure")).toBe(
            "Connection to the group was lost. Rejoin to continue.",
        );
    });

    // The server writes names, and can make two alike. A code is placed only by the socket the
    // card is for, never by name.
    it("two_code_notices_with_the_same_peer_name_land_on_their_own_cards", async () => {
        const manager = await started();
        deliver("protected-group:request", { joiner_socket: 7, name: "minecraft:Steve" });
        deliver("protected-group:request", { joiner_socket: 9, name: "minecraft:Steve" });

        deliver("protected-group:code", {
            role: "member",
            check_id: "aa".repeat(16),
            joiner_socket: 9,
            peer_name: "minecraft:Steve",
            code: "1111 2222",
        });

        const cards = get(manager.cards);
        expect(cards.find((c) => c.joinerSocket === 9)?.code).toBe("1111 2222");
        expect(cards.find((c) => c.joinerSocket === 7)?.code).toBeNull();
    });

    it("a_new_check_id_for_a_card_showing_a_live_code_is_ignored", async () => {
        const manager = await started();
        deliver("protected-group:request", { joiner_socket: 7, name: "minecraft:Steve" });
        const code = (check_id: string, value: string) =>
            deliver("protected-group:code", {
                role: "member",
                check_id,
                joiner_socket: 7,
                peer_name: "minecraft:Steve",
                code: value,
            });

        code("aa".repeat(16), "1111 2222");
        code("bb".repeat(16), "3333 4444");

        const card = get(manager.cards).find((c) => c.joinerSocket === 7);
        expect(card?.checkId).toBe("aa".repeat(16));
        expect(card?.code).toBe("1111 2222");
    });

    it("decide_sends_the_cards_own_check_id", async () => {
        mockInvoke({ group_admission_decide: () => null });
        const manager = await started();
        deliver("protected-group:request", { joiner_socket: 7, name: "minecraft:Steve" });
        deliver("protected-group:code", {
            role: "member",
            check_id: "aa".repeat(16),
            joiner_socket: 7,
            peer_name: "minecraft:Steve",
            code: "1111 2222",
        });

        await manager.decide(7, "aa".repeat(16), true);

        const call = invokeCalls().find((c) => c.cmd === "group_admission_decide");
        expect(call?.args).toEqual({ joinerSocket: 7, checkId: "aa".repeat(16), allow: true });
    });

    it("a_checking_notice_never_replaces_a_card_showing_a_code", async () => {
        const manager = await started();
        deliver("protected-group:request", { joiner_socket: 7, name: "minecraft:Steve" });
        deliver("protected-group:code", {
            role: "member",
            check_id: "aa".repeat(16),
            joiner_socket: 7,
            peer_name: "minecraft:Steve",
            code: "1111 2222",
        });

        deliver("protected-group:checking", { joiner_socket: 7, member_name: "minecraft:Bob" });

        const card = get(manager.cards).find((c) => c.joinerSocket === 7);
        expect(card?.phase).toBe("code");
        expect(card?.code).toBe("1111 2222");
    });

    it("a_card_closes_when_its_request_ends", async () => {
        const manager = await started();
        deliver("protected-group:request", { joiner_socket: 7, name: "minecraft:Steve" });

        deliver("protected-group:request-ended", { joiner_socket: 7 });

        expect(get(manager.cards)).toEqual([]);
    });

    it("the_join_flow_goes_from_asking_to_a_code_to_joined", async () => {
        mockInvoke({
            group_join_protected: () => ({ in_group: true, id: "NR4K", name: null }),
            group_admission_confirm: () => null,
        });
        const manager = await started();

        manager.openJoin("NR4K");
        expect(get(manager.joinFlow)?.phase).toBe("ask");
        await manager.ask();
        expect(get(manager.joinFlow)?.phase).toBe("waiting");
        deliver("protected-group:code", {
            role: "joiner",
            check_id: "cc".repeat(16),
            joiner_socket: 3,
            peer_name: "minecraft:AL",
            code: "4821 0937",
        });
        expect(get(manager.joinFlow)).toMatchObject({
            phase: "code",
            code: "4821 0937",
            checkId: "cc".repeat(16),
            checker: "minecraft:AL",
        });

        await manager.confirm(true);
        expect(get(manager.joinFlow)?.phase).toBe("joining");
        const call = invokeCalls().find((c) => c.cmd === "group_admission_confirm");
        expect(call?.args).toEqual({ checkId: "cc".repeat(16), matches: true });

        deliver("protected-group:joined", { epoch: 1, safety_code: "x" });
        expect(get(manager.joinFlow)).toBeNull();
    });

    // "It matches" releases the held Welcome, so the join can finish before the confirm call
    // returns. The dialog must not come back as "Joining" after the joined event closed it.
    it("a_join_that_finishes_before_confirm_returns_closes_the_dialog", async () => {
        mockInvoke({
            group_join_protected: () => ({ in_group: true, id: "NR4K", name: null }),
            group_admission_confirm: () => {
                deliver("protected-group:joined", { epoch: 1, safety_code: "x" });
                return null;
            },
        });
        const manager = await started();
        manager.openJoin("NR4K");
        await manager.ask();
        deliver("protected-group:code", {
            role: "joiner",
            check_id: "cc".repeat(16),
            joiner_socket: 3,
            peer_name: "minecraft:AL",
            code: "4821 0937",
        });

        await manager.confirm(true);

        expect(get(manager.joinFlow)).toBeNull();
    });

    // A confirm the app could not send leaves the code on screen to answer again.
    it("a_failed_confirm_keeps_the_code_on_screen", async () => {
        mockInvoke({
            group_join_protected: () => ({ in_group: true, id: "NR4K", name: null }),
            group_admission_confirm: () => {
                throw "no ticket";
            },
        });
        const manager = await started();
        manager.openJoin("NR4K");
        await manager.ask();
        deliver("protected-group:code", {
            role: "joiner",
            check_id: "cc".repeat(16),
            joiner_socket: 3,
            peer_name: "minecraft:AL",
            code: "4821 0937",
        });

        await manager.confirm(true);

        expect(get(manager.joinFlow)).toMatchObject({ phase: "code", code: "4821 0937" });
    });

    it("a_refusal_while_asking_shows_in_the_dialog", async () => {
        mockInvoke({ group_join_protected: () => ({ in_group: true, id: "NR4K", name: null }) });
        const manager = await started();
        manager.openJoin("NR4K");
        await manager.ask();

        deliver("protected-group:lost", { reason: "refused" });

        expect(get(manager.joinFlow)?.phase).toBe("refused");
        expect(get(manager.lostReason)).toBeNull();
    });

    it("a_join_from_the_game_opens_the_dialog_waiting", async () => {
        const manager = await started();

        deliver("protected-group:join-started", { channel_id: "NR4K" });

        expect(get(manager.joinFlow)).toMatchObject({ id: "NR4K", phase: "waiting" });
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

    it("protected_join_invokes_group_join_protected_with_the_id_only", async () => {
        mockInvoke({ group_join_protected: () => ({ in_group: true, id: "AB12CD34", name: null }) });
        const manager = await started();

        await manager.join("AB12CD34");

        const call = invokeCalls().find((c) => c.cmd === "group_join_protected");
        expect(call?.args).toEqual({ id: "AB12CD34" });
    });

    it("protect_maps_the_servers_refusal", async () => {
        const manager = await started();

        mockInvoke({ group_protect: () => ({ in_group: true, id: "AB12CD34", name: null }) });
        expect(await manager.protect("AB12CD34")).toBe("ok");

        mockInvoke({
            group_protect: () => {
                throw "not_alone";
            },
        });
        expect(await manager.protect("AB12CD34")).toBe("not_alone");

        mockInvoke({
            group_protect: () => {
                throw "already_protected";
            },
        });
        expect(await manager.protect("AB12CD34")).toBe("already_protected");

        mockInvoke({
            group_protect: () => {
                throw "the socket broke";
            },
        });
        expect(await manager.protect("AB12CD34")).toBe("failed");
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
            "protected-group:admitted",
            "protected-group:check-failed",
            "protected-group:checking",
            "protected-group:code",
            "protected-group:epoch",
            "protected-group:join-started",
            "protected-group:joined",
            "protected-group:lost",
            "protected-group:request",
            "protected-group:request-ended",
            "protected-group:roster",
            "protected-group:waiting",
        ]);
    });
});
