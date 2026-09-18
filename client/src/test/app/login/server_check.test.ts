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
    mockInvoke({ check_server: () => ({ client_id: "abc", status: "ok" }) });

    await ServerCheck.config(SERVER);

    expect(invokeCalls()).toEqual([{ cmd: "check_server", args: { server: SERVER } }]);
  });

  it("returns the configuration the command answered with", async () => {
    mockInvoke({ check_server: () => ({ client_id: "abc", status: "ok" }) });

    const config = await ServerCheck.config(SERVER);

    expect(config.client_id).toBe("abc");
  });

  // The screen's error view is driven by the rejection. Swallowing it here would put the
  // player back on a form that looks like it worked.
  it("propagates a failure from the command", async () => {
    mockInvoke({
      check_server: () => {
        throw new Error("Login request to https://example.invalid/api/config failed: operation timed out");
      },
    });

    await expect(ServerCheck.config(SERVER)).rejects.toThrow("operation timed out");
  });
});
