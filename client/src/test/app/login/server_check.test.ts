import { beforeEach, describe, expect, it } from "vitest";
import { mockInvoke, invokeCalls } from "../../tauri";
import { ServerCheck } from "../../../js/app/login/ServerCheck";

const SERVER = "https://example.invalid";

/**
 * The sign-in screen's availability check and the sign-in that follows it were two
 * different HTTP clients: a webview fetch through the HTTP plugin, then a separately built
 * Rust client. They can disagree on TLS, address family and timeout, so the check could
 * answer for a server the sign-in could not reach — which is what put "the server is
 * available" in front of a player whose sign-in then failed.
 */
describe("ServerCheck", () => {
    beforeEach(() => {
        mockInvoke({});
    });

    it("asks Rust rather than fetching from the webview", async () => {
        mockInvoke({
            check_server: () => ({ state: "Reachable", config: { client_id: "abc", status: "ok" } }),
        });

        await ServerCheck.report(SERVER);

        expect(invokeCalls()).toEqual([{ cmd: "check_server", args: { server: SERVER } }]);
    });

    it("returns the configuration the command answered with", async () => {
        mockInvoke({
            check_server: () => ({ state: "Reachable", config: { client_id: "abc", status: "ok" } }),
        });

        const report = await ServerCheck.report(SERVER);

        expect(report.state === "Reachable" && report.config.client_id).toBe("abc");
    });

    // The classified failure is what the screen shows, so it must arrive intact.
    it("returns a classified failure unchanged", async () => {
        const failed = {
            state: "Failed",
            failure: { kind: "TimedOut" },
            detail: "Login request to https://example.invalid/api/config failed: operation timed out",
        };
        mockInvoke({ check_server: () => failed });

        expect(await ServerCheck.report(SERVER)).toEqual(failed);
    });

    it("propagates a fault from the command itself", async () => {
        mockInvoke({
            check_server: () => {
                throw new Error("state unavailable");
            },
        });

        await expect(ServerCheck.report(SERVER)).rejects.toThrow("state unavailable");
    });
});
