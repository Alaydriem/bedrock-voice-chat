import { beforeEach, describe, expect, it, vi } from "vitest";
import { mockInvoke, invokeCalls } from "../tauri";
import { AuthCallbackHandler } from "../../js/app/deepLinkHandlers/authCallbackHandler";

vi.mock("../../js/app/analytics", () => ({
  default: { track: vi.fn() },
}));

/**
 * An authorization code is single-use, and this app has several ways to present the same
 * one: every deep-link intent writes `pending_deep_link` *and* emits `deep-link-received`,
 * and on Android an intent arrives while the app is backgrounded or cold. The callback
 * therefore lands either side of a page load, and the router's in-memory record of what it
 * has already routed does not survive one.
 *
 * The second exchange spends a code the provider has retired. It answers that the code is
 * invalid, the server reports the login as failed, and the screen tells someone their
 * account was refused — for a code that worked the first time.
 *
 * A fresh handler on a shared store is the test's stand-in for that second page: same
 * device, same store, no memory.
 */

const STATE = "state-token";
const SERVER = "https://s4.example.com";

function fakeStore(seed: Record<string, unknown> = {}) {
  const data = new Map<string, unknown>(Object.entries(seed));
  return {
    get: vi.fn(async (key: string) => data.get(key)),
    set: vi.fn(async (key: string, value: unknown) => void data.set(key, value)),
    delete: vi.fn(async (key: string) => void data.delete(key)),
    save: vi.fn(async () => {}),
    has: vi.fn(async (key: string) => data.has(key)),
  };
}

function store() {
  return fakeStore({ auth_state_token: STATE, auth_state_endpoint: SERVER });
}

function callback(code: string): string {
  return `bedrock-voice-chat://auth/?code=${code}&state=${STATE}`;
}

function loginAttempts(): number {
  return invokeCalls().filter((call) => call.cmd === "server_login").length;
}

describe("exchanging an authorization code", () => {
  beforeEach(() => {
    // Keeps failLogin from navigating: it only redirects when the callback landed
    // somewhere other than the login page.
    window.history.replaceState({}, "", "/login");
    mockInvoke({
      server_login: () => {
        throw new Error("401 Unauthorized: the sign-in did not complete");
      },
    });
  });

  it("sends the code for exchange", async () => {
    const handler = new AuthCallbackHandler(store() as never);
    await handler.handle(callback("CODE_A"));
    expect(loginAttempts()).toBe(1);
  });

  it("does not send the same code again from a fresh page", async () => {
    const shared = store();

    await new AuthCallbackHandler(shared as never).handle(callback("CODE_B"));
    await new AuthCallbackHandler(shared as never).handle(callback("CODE_B"));

    expect(loginAttempts()).toBe(1);
  });

  /**
   * Claimed before the exchange, not after. A code is spent the moment it is sent, so a
   * failed exchange must not leave it looking available — retrying it cannot succeed, and
   * the failure it produces is the one that reads as a refused account.
   */
  it("treats a code as spent even when the exchange failed", async () => {
    const shared = store();
    const handler = new AuthCallbackHandler(shared as never);

    await handler.handle(callback("CODE_C"));
    await handler.handle(callback("CODE_C"));

    expect(loginAttempts()).toBe(1);
  });

  it("still exchanges a code from a second sign-in", async () => {
    const shared = store();

    await new AuthCallbackHandler(shared as never).handle(callback("CODE_D"));
    await new AuthCallbackHandler(shared as never).handle(callback("CODE_E"));

    expect(loginAttempts()).toBe(2);
  });

  /**
   * The guard keeps a bounded history. Older codes falling out of it is intended — they are
   * long expired — but the sign-in in front of the user must never be the one evicted.
   */
  it("keeps the most recent codes, not the oldest", async () => {
    const shared = store();

    for (const code of ["C1", "C2", "C3", "C4", "C5", "C6"]) {
      await new AuthCallbackHandler(shared as never).handle(callback(code));
    }
    expect(loginAttempts()).toBe(6);

    await new AuthCallbackHandler(shared as never).handle(callback("C6"));
    expect(loginAttempts()).toBe(6);
  });

  it("clears the pending callback so the next launch does not replay it", async () => {
    const shared = store();

    await new AuthCallbackHandler(shared as never).handle(callback("CODE_F"));
    await new AuthCallbackHandler(shared as never).handle(callback("CODE_F"));

    expect(shared.delete).toHaveBeenCalledWith("pending_deep_link");
  });

  // The state is compared to what was stored when the sign-in opened. A callback from some
  // other attempt must not spend a code against this one.
  it("does not exchange a callback whose state does not match", async () => {
    const handler = new AuthCallbackHandler(store() as never);
    await handler.handle(`bedrock-voice-chat://auth/?code=CODE_G&state=someone-elses`);
    expect(loginAttempts()).toBe(0);
  });

  /**
   * A successful login deletes the state token, so a duplicate intent arriving afterwards
   * cannot pass the state comparison. Recognised as a duplicate it is silently dropped;
   * treated as a state mismatch it persists a login error and sends someone who is signed
   * in and part-way through setup back to the login page.
   */
  it("ignores a duplicate that arrives after the login succeeded", async () => {
    const spent = fakeStore({ redeemed_auth_codes: ["CODE_H"], auth_state_endpoint: SERVER });

    await new AuthCallbackHandler(spent as never).handle(callback("CODE_H"));

    expect(loginAttempts()).toBe(0);
    expect(spent.set).not.toHaveBeenCalledWith("login_error", expect.anything());
  });
});
