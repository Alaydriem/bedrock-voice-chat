import { describe, expect, it, vi } from "vitest";
import LaunchGate from "../../../js/app/login/LaunchGate";

const none = new URLSearchParams("");

describe("LaunchGate.resolveEntry", () => {
  // Someone who has not read the introduction has not been onboarded. Defaulting the other
  // way sends a brand-new user to a credential prompt for a server they do not have.
  it("onboards an install that has not seen it", () => {
    expect(LaunchGate.resolveEntry(false, none)).toBe("intro");
  });

  it("goes straight to sign in once it has been seen", () => {
    expect(LaunchGate.resolveEntry(true, none)).toBe("login");
  });

  it("skips the introduction when adding a server", () => {
    expect(LaunchGate.resolveEntry(true, new URLSearchParams("addserver"))).toBe("login");
  });

  it("skips the introduction when re-authenticating a known server", () => {
    expect(
      LaunchGate.resolveEntry(true, new URLSearchParams("server=bvc.example.com&reauth=true")),
    ).toBe("login");
  });

  // Belt and braces: an install arriving with these params has necessarily been through
  // this, but a future caller must not be able to drop someone into an explainer mid-task.
  it("never onboards a launch that arrived with a server in hand", () => {
    expect(LaunchGate.resolveEntry(false, new URLSearchParams("server=bvc.example.com"))).toBe(
      "login",
    );
  });

  /**
   * The behaviour this marker exists for. Signing out empties the server list, and the
   * introduction is not part of signing back in — someone who has read it should reach the
   * credential prompt directly.
   */
  it("does not re-onboard after a logout", () => {
    expect(LaunchGate.resolveEntry(true, new URLSearchParams("logout=true"))).toBe("login");
  });
});

function fakeStore(seed: Record<string, unknown> = {}) {
  const data = new Map<string, unknown>(Object.entries(seed));
  return {
    get: vi.fn(async (key: string) => data.get(key)),
    set: vi.fn(async (key: string, value: unknown) => void data.set(key, value)),
    save: vi.fn(async () => {}),
    read: () => data,
  };
}

let store = fakeStore();
vi.mock("@tauri-apps/plugin-store", () => ({
  Store: { load: vi.fn(async () => store) },
}));

function gate(servers: string[], seed: Record<string, unknown> = {}) {
  store = fakeStore(seed);
  const serverListStore = {
    getServerList: vi.fn(async () => servers.map((server) => ({ server, player: "p" }))),
  };
  return new LaunchGate(serverListStore as never);
}

describe("whether the introduction has been seen", () => {
  it("is unseen on a fresh install", async () => {
    expect(await gate([]).hasSeenOnboarding()).toBe(false);
  });

  it("is seen once the marker is set", async () => {
    expect(await gate([], { onboarding_seen: true }).hasSeenOnboarding()).toBe(true);
  });

  /**
   * An install that predates the marker has a server list and nothing else. Reading that as
   * unseen would hand the introduction to someone who has been using the app for months.
   */
  it("counts an existing server list as having been through it", async () => {
    expect(await gate(["bvc.example.com"]).hasSeenOnboarding()).toBe(true);
  });

  // The marker survives signing out, which is the whole point of storing it rather than
  // inferring it from the server list.
  it("stays seen when the server list is emptied", async () => {
    const marked = gate([], { onboarding_seen: true });
    expect(await marked.hasSeenOnboarding()).toBe(true);
  });

  it("records having been read", async () => {
    const fresh = gate([]);
    await fresh.markSeen();
    expect(store.read().get("onboarding_seen")).toBe(true);
    expect(store.save).toHaveBeenCalled();
  });

  /**
   * Losing this costs one repeat of the introduction. Throwing out of the handler that calls
   * it would cost the button press that was leaving the introduction.
   */
  it("does not fail the flow when it cannot be recorded", async () => {
    const fresh = gate([]);
    store.set = vi.fn(async () => {
      throw new Error("store is read-only");
    });
    await expect(fresh.markSeen()).resolves.toBeUndefined();
  });
});
