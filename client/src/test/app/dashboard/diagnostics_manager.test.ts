import { get } from "svelte/store";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { mockInvoke } from "../../tauri";
import type { ConnectionHealth } from "../../../js/bindings/ConnectionHealth";

const listeners = new Map<string, (event: { payload: unknown }) => void>();

vi.mock("@tauri-apps/api/webviewWindow", () => ({
    getCurrentWebviewWindow: () => ({
        listen: async (event: string, run: (e: { payload: unknown }) => void) => {
            listeners.set(event, run);
            return () => listeners.delete(event);
        },
    }),
}));

const { DiagnosticsManager } = await import("../../../js/app/dashboard/DiagnosticsManager");

/**
 * @param snapshot what `get_link_diagnostics` answers. Null is not "no data yet": the backend
 *   returns nothing precisely while the session is disconnected, so it is the seed for health.
 */
async function started(snapshot: unknown = null) {
    mockInvoke({ get_link_diagnostics: () => snapshot });
    const manager = new DiagnosticsManager();
    await manager.start();
    return manager;
}

function report(health: ConnectionHealth): void {
    listeners.get("connection_health")?.({ payload: health });
}

/**
 * The decode, through the store it feeds.
 *
 * This existed as a private static reading `'Reconnecting' in health` — and `ConnectionHealth` is
 * a *tagged* union, `{ status: "Reconnecting", attempt }`, with no `Reconnecting` key. The test
 * was always false, so `reconnecting` was permanently false and every unhealthy status collapsed
 * into a link reading as fine. Nothing failed; the dashboard simply never learned it was down.
 */
describe("DiagnosticsManager health", () => {
    beforeEach(() => {
        listeners.clear();
    });

    // `connection_health` arrives on change rather than on a clock, so a link that was already up
    // when this mounted has no event left to send. A snapshot is what says so.
    it("takes a snapshot as evidence the link is up", async () => {
        const manager = await started({ link: {} });

        expect(get(manager.health).connected).toBe(true);
    });

    /**
     * The dashboard does not only mount after a connection.
     *
     * A webview reload leaves the Rust side running, so the dashboard rebuilds nothing and no
     * health event is outstanding to correct an optimistic default. Assuming up meant a reload
     * over a dead link drew a full roster of people who could not hear a word — the one thing
     * this screen must never assert. The backend answers nothing at all while disconnected, which
     * makes absence positive evidence rather than missing data.
     */
    it("treats an absent snapshot as the link being down", async () => {
        const manager = await started(null);

        const health = get(manager.health);
        expect(health.connected).toBe(false);
        expect(health.reconnecting).toBe(false);
    });

    it("reports a reconnect, counting attempts from one", async () => {
        const manager = await started();
        report({ status: "Reconnecting", attempt: 0 });

        const health = get(manager.health);
        expect(health.connected).toBe(false);
        expect(health.reconnecting).toBe(true);
        // Zero on the wire; a verdict reading "attempt 0" looks like a bug, not a first try.
        expect(health.attempt).toBe(1);
    });

    it("reports a plain disconnect as down but not retrying", async () => {
        const manager = await started();
        report({ status: "Disconnected" });

        expect(get(manager.health)).toMatchObject({ connected: false, reconnecting: false });
    });

    // Each of these used to read as a healthy link.
    it.each([
        ["Failed", { status: "Failed" } as ConnectionHealth],
        [
            "VersionMismatch",
            {
                status: "VersionMismatch",
                client_version: "1.0.0",
                server_version: "1.2.0",
                client_too_old: true,
            } as ConnectionHealth,
        ],
        ["Unauthorized", { status: "Unauthorized", reason: "not whitelisted" } as ConnectionHealth],
    ])("treats %s as down, and says why it will not recover", async (_name, payload) => {
        const manager = await started();
        report(payload);

        const health = get(manager.health);
        expect(health.connected).toBe(false);
        expect(health.reconnecting).toBe(false);
        expect(health.fatal).toBeTruthy();
    });

    it("comes back up on a reconnect", async () => {
        const manager = await started();
        report({ status: "Disconnected" });
        expect(get(manager.health).connected).toBe(false);

        report({ status: "Connected" });
        expect(get(manager.health)).toMatchObject({ connected: true, reconnecting: false });
    });

    // An unrecognised tag is not evidence of a broken link, and blanking the roster on one would
    // be a worse failure than ignoring it.
    it("keeps the link up on a status it does not recognise", async () => {
        const manager = await started();
        report({ status: "SomethingNew" } as unknown as ConnectionHealth);

        expect(get(manager.health).connected).toBe(true);
    });
});
