import { describe, expect, it, vi } from "vitest";
import { invokeCalls, mockInvoke } from "../../tauri";
import { ServerHealthService } from "../../../js/app/services/ServerHealthService";

/**
 * The public config read, which is the only thing this service does outside the IPC
 * boundary. `answering` decides whether the server is up.
 */
let answering = true;
vi.mock("@tauri-apps/plugin-http", () => ({
  fetch: vi.fn(async () => {
    if (!answering) throw new Error("connection refused");
    return { status: 200, json: async () => ({ protocol_version: "2.1.0", quic_ports: [443] }) };
  }),
}));

const CREDENTIALS = {
  certificate_ca: "ca",
  certificate: "cert",
  certificate_key: "key",
};

const SERVER = "https://bvc.example.com";

function ipc(overrides: Record<string, (args: never) => unknown> = {}) {
  mockInvoke({
    get_credentials: () => CREDENTIALS,
    is_certificate_expired: () => false,
    api_pool_client: () => null,
    api_get_config: () => ({
      config: { protocol_version: "2.1.0" },
      client_version: "2.1.0",
      compatible: true,
      client_too_old: false,
    }),
    ...overrides,
  });
}

describe("ServerHealthService", () => {
  it("reports a server whose credentials still work", async () => {
    ipc();
    const result = await new ServerHealthService().check(SERVER);
    expect(result.status).toBe("connect");
  });

  /**
   * The reason this service exists in its own right rather than inline in the page: it must
   * never claim the session. `current_server` decides which server `logout` clears and
   * which one commands with no explicit endpoint act on, so a sweep across every saved
   * server would otherwise leave it pointing at whichever check finished last.
   */
  it("makes a server callable without claiming it as the current one", async () => {
    ipc();
    await new ServerHealthService().check(SERVER);

    const commands = invokeCalls().map((call) => call.cmd);
    expect(commands).toContain("api_pool_client");
    expect(commands).not.toContain("api_initialize_client");
  });

  // Rotating a certificate is a write. Looking at a list of servers is not a reason to do
  // it for all of them.
  it("does not rotate credentials while checking", async () => {
    ipc();
    await new ServerHealthService().check(SERVER);
    expect(invokeCalls().map((call) => call.cmd)).not.toContain("refresh_server_state");
  });

  it("reports no credentials saved when the keyring has none", async () => {
    ipc({
      get_credentials: () => {
        throw new Error("no entry");
      },
    });
    expect((await new ServerHealthService().check(SERVER)).status).toBe("missing");
  });

  it("reports a lapsed sign-in for an expired certificate without calling the server", async () => {
    ipc({ is_certificate_expired: () => true });
    const result = await new ServerHealthService().check(SERVER);
    expect(result.status).toBe("reauth");
    expect(invokeCalls().map((call) => call.cmd)).not.toContain("api_get_config");
  });

  /**
   * The misdiagnosis this fixes. A server that is simply down used to come back as "auth
   * required", which sends someone to a Microsoft sign-in that cannot succeed and makes a
   * server outage look like their fault.
   */
  it("distinguishes a server that is down from credentials that were refused", async () => {
    answering = false;
    ipc({
      api_get_config: () => {
        throw new Error("connection refused");
      },
    });
    expect((await new ServerHealthService().check(SERVER)).status).toBe("unreachable");
  });

  it("reports a lapsed sign-in when the server is up but refused the call", async () => {
    answering = true;
    ipc({
      api_get_config: () => {
        throw new Error("certificate rejected");
      },
    });
    expect((await new ServerHealthService().check(SERVER)).status).toBe("reauth");
  });

  it("names both versions when the protocols do not match", async () => {
    ipc({
      api_get_config: () => ({
        config: { protocol_version: "2.2.0" },
        client_version: "2.1.0",
        compatible: false,
        client_too_old: true,
      }),
    });
    const result = await new ServerHealthService().check(SERVER);
    expect(result.status).toBe("version_mismatch");
    expect(result.serverVersion).toBe("2.2.0");
    expect(result.clientVersion).toBe("2.1.0");
    expect(result.clientTooOld).toBe(true);
  });

  // A row has to say something. Every failure is a status, so nothing reaches the caller
  // as an exception.
  it("never throws, whatever the boundary does", async () => {
    answering = false;
    mockInvoke({
      get_credentials: () => CREDENTIALS,
      is_certificate_expired: () => {
        throw new Error("keyring locked");
      },
    });
    await expect(new ServerHealthService().check(SERVER)).resolves.toBeDefined();
  });
});
