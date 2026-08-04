import { beforeEach, describe, expect, it, vi } from "vitest";
import { get } from "svelte/store";
import { ServerRosterManager } from "../../../js/app/server/ServerRosterManager";
import type { ServerHealthResult } from "../../../js/app/services/ServerHealthResult";
import type { ServerHealthStatus } from "../../../js/app/services/ServerHealthStatus";

function health(status: ServerHealthStatus, overrides: Partial<ServerHealthResult> = {}) {
  return {
    status,
    compatible: status === "connect",
    clientTooOld: false,
    serverVersion: "",
    clientVersion: "",
    ...overrides,
  } as ServerHealthResult;
}

interface Fixture {
  readonly servers: string[];
  readonly current?: string | null;
  readonly checks?: Record<string, ServerHealthResult>;
  readonly update?: string | null;
}

function build(fixture: Fixture) {
  const list = fixture.servers.map((server) => ({ server, player: "Alaydriem", game: "minecraft" }));

  const serverList = {
    getServerList: vi.fn(async () => list),
    getCurrentServer: vi.fn(async () => fixture.current ?? null),
    setCurrent: vi.fn(async () => {}),
    removeServer: vi.fn(async (server: string) => list.filter((e) => e.server !== server)),
  };

  const check = vi.fn(async (server: string) => fixture.checks?.[server] ?? health("connect"));
  const forgetCredentials = vi.fn(async () => {});
  const checkForUpdates = vi.fn(async () => fixture.update ?? null);

  const manager = new ServerRosterManager({
    health: { check } as never,
    serverList: serverList as never,
    forgetCredentials,
    checkForUpdates,
  });

  return { manager, serverList, check, forgetCredentials, checkForUpdates };
}

describe("loading the list", () => {
  it("draws every saved server before any of them has been checked", async () => {
    const { manager, check } = build({ servers: ["https://a", "https://b"] });
    const count = await manager.load();

    expect(count).toBe(2);
    expect(get(manager.entries).map((e) => e.status)).toEqual(["checking", "checking"]);
    expect(check).not.toHaveBeenCalled();
  });

  it("shows the host without its scheme, which is what a person recognises", async () => {
    const { manager } = build({ servers: ["https://bvc.example.com/"] });
    await manager.load();
    expect(get(manager.entries)[0].host).toBe("bvc.example.com");
  });

  it("ticks the server this app was last signed in to", async () => {
    const { manager } = build({ servers: ["https://a", "https://b"], current: "https://b" });
    await manager.load();
    expect(get(manager.entries).map((e) => e.isCurrent)).toEqual([false, true]);
  });

  it("carries each check's answer onto its own row", async () => {
    const { manager } = build({
      servers: ["https://a", "https://b"],
      checks: { "https://a": health("connect"), "https://b": health("unreachable") },
    });
    await manager.load();
    await manager.sweep();
    expect(get(manager.entries).map((e) => e.status)).toEqual(["connect", "unreachable"]);
  });

  /**
   * The reason the sweep is separate from the draw. A dead host takes as long as its
   * timeout, and holding every other row behind it would make one broken server look like
   * a broken app.
   */
  it("checks servers concurrently rather than one after another", async () => {
    let running = 0;
    let peak = 0;
    const check = vi.fn(async () => {
      running++;
      peak = Math.max(peak, running);
      await Promise.resolve();
      running--;
      return health("connect");
    });

    const manager = new ServerRosterManager({
      health: { check } as never,
      serverList: {
        getServerList: async () => [
          { server: "https://a", player: "p" },
          { server: "https://b", player: "p" },
          { server: "https://c", player: "p" },
        ],
        getCurrentServer: async () => null,
      } as never,
      forgetCredentials: async () => {},
      checkForUpdates: async () => null,
    });

    await manager.load();
    await manager.sweep();
    expect(peak).toBe(3);
  });
});

describe("choosing a server", () => {
  it("claims the server before handing over to the dashboard", async () => {
    const { manager, serverList } = build({ servers: ["https://a"] });
    await manager.load();
    await manager.sweep();

    const next = await manager.choose("https://a");

    expect(serverList.setCurrent).toHaveBeenCalledWith({
      server: "https://a",
      player: "Alaydriem",
      game: "minecraft",
    });
    expect(next).toEqual({ kind: "navigate", href: "/dashboard?server=https://a" });
  });

  it("sends a lapsed sign-in back to login for that server", async () => {
    const { manager } = build({
      servers: ["https://a"],
      checks: { "https://a": health("reauth") },
    });
    await manager.load();
    await manager.sweep();

    expect(await manager.choose("https://a")).toEqual({
      kind: "navigate",
      href: "/login?reauth=true&server=https://a",
    });
  });

  // Nothing about a server that is down is fixed by signing in or by leaving the page.
  it("re-checks a server that was not answering rather than navigating", async () => {
    const { manager, check } = build({
      servers: ["https://a"],
      checks: { "https://a": health("unreachable") },
    });
    await manager.load();
    await manager.sweep();
    check.mockClear();

    expect(await manager.choose("https://a")).toEqual({ kind: "none" });
    expect(check).toHaveBeenCalledWith("https://a");
  });

  it("goes nowhere while a check is still running", async () => {
    const { manager } = build({ servers: ["https://a"] });
    await manager.load();
    expect(await manager.choose("https://a")).toEqual({ kind: "none" });
  });

  it("goes to the update screen when this build is behind and an update exists", async () => {
    const { manager } = build({
      servers: ["https://a"],
      checks: { "https://a": health("version_mismatch", { clientTooOld: true }) },
      update: "1.0.0-beta.9",
    });
    await manager.load();
    await manager.sweep();

    expect(await manager.choose("https://a")).toEqual({
      kind: "navigate",
      href: "/error?code=UPD01&version=1.0.0-beta.9",
    });
  });

  /**
   * An update screen with no update to install is a dead end. Saying so on the row is the
   * honest answer, and it is also the true one: the server is ahead of every build.
   */
  it("says so on the row when this build is behind and no update exists", async () => {
    const { manager } = build({
      servers: ["https://a"],
      checks: { "https://a": health("version_mismatch", { clientTooOld: true }) },
      update: null,
    });
    await manager.load();
    await manager.sweep();

    expect(await manager.choose("https://a")).toEqual({ kind: "none" });
    expect(get(manager.entries)[0].note).toMatch(/no update/i);
  });

  it("does not offer to update past a server running an older protocol", async () => {
    const { manager, checkForUpdates } = build({
      servers: ["https://a"],
      checks: { "https://a": health("version_mismatch", { clientTooOld: false }) },
    });
    await manager.load();
    await manager.sweep();

    expect(await manager.choose("https://a")).toEqual({ kind: "none" });
    expect(checkForUpdates).not.toHaveBeenCalled();
  });
});

describe("forgetting a server", () => {
  it("clears the credentials as well as the list entry", async () => {
    const { manager, serverList, forgetCredentials } = build({
      servers: ["https://a", "https://b"],
    });
    await manager.load();

    await manager.remove("https://a");

    expect(forgetCredentials).toHaveBeenCalledWith("https://a");
    expect(serverList.removeServer).toHaveBeenCalledWith("https://a");
    expect(get(manager.entries).map((e) => e.server)).toEqual(["https://b"]);
  });

  // A list with nothing in it is not a page worth showing.
  it("returns to sign-in once the last server is gone", async () => {
    const { manager } = build({ servers: ["https://a"] });
    await manager.load();
    expect(await manager.remove("https://a")).toEqual({ kind: "navigate", href: "/login" });
  });

  /**
   * Credentials that outlive their row are invisible and stay on the device, so a keyring
   * that refuses must not stop the entry being removed.
   */
  it("still removes the entry when the credentials cannot be cleared", async () => {
    const { manager, serverList } = build({ servers: ["https://a", "https://b"] });
    const failing = new ServerRosterManager({
      health: { check: async () => health("connect") } as never,
      serverList: serverList as never,
      forgetCredentials: async () => {
        throw new Error("keyring locked");
      },
      checkForUpdates: async () => null,
    });
    await failing.load();

    await failing.remove("https://a");
    expect(serverList.removeServer).toHaveBeenCalledWith("https://a");
  });
});

describe("ServerRosterManager.autoJoin", () => {
  function row(status: ServerHealthStatus, server = "https://a") {
    return {
      server,
      host: server,
      player: "p",
      game: "minecraft",
      status,
      serverVersion: "",
      clientVersion: "",
      clientTooOld: false,
      isCurrent: false,
    };
  }

  it("skips the list when the only saved server is ready", () => {
    expect(ServerRosterManager.autoJoin([row("connect")])).toBe("https://a");
  });

  // Everything the list would have said about a single broken server is worth saying.
  it("shows the list when the only saved server needs something", () => {
    for (const status of ["reauth", "missing", "unreachable", "version_mismatch"] as const) {
      expect(ServerRosterManager.autoJoin([row(status)])).toBeNull();
    }
  });

  it("never skips a choice between two servers", () => {
    expect(ServerRosterManager.autoJoin([row("connect"), row("connect", "https://b")])).toBeNull();
  });
});

describe("refreshing", () => {
  let flag: boolean[] = [];
  beforeEach(() => (flag = []));

  it("reports it is working while every server is re-checked", async () => {
    const { manager } = build({ servers: ["https://a"] });
    manager.isRefreshing.subscribe((v) => flag.push(v));

    await manager.refreshAll();

    expect(flag).toEqual([false, true, false]);
  });
});
