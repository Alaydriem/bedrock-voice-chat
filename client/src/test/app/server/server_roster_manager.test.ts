import { beforeEach, describe, expect, it, vi } from "vitest";
import { get } from "svelte/store";
import { ServerRosterManager } from "../../../js/app/server/ServerRosterManager";
import { PreflightRunner } from "../../../js/app/server/preflight/PreflightRunner";
import type { PreflightOutcome } from "../../../js/app/server/preflight/PreflightOutcome";

function outcome(
  status: PreflightOutcome["status"],
  overrides: Partial<PreflightOutcome> = {},
): PreflightOutcome {
  return {
    status,
    rtt: 24,
    slow: false,
    quicPort: 443,
    serverVersion: "2.1.0",
    clientVersion: "2.1.0",
    clientTooOld: false,
    ...overrides,
  };
}

interface Fixture {
  readonly servers: string[];
  readonly outcomes?: Record<string, PreflightOutcome>;
  readonly update?: string | null;
}

function build(fixture: Fixture) {
  const list = fixture.servers.map((server) => ({
    server,
    player: "Alaydriem",
    game: "minecraft",
  }));

  const serverList = {
    getServerList: vi.fn(async () => list),
    setCurrent: vi.fn(async () => {}),
    removeServer: vi.fn(async (server: string) => list.filter((e) => e.server !== server)),
  };

  const preflight = vi.fn(async (server: string) => fixture.outcomes?.[server] ?? outcome("connect"));
  const forgetCredentials = vi.fn(async () => {});
  const checkForUpdates = vi.fn(async () => fixture.update ?? null);
  const getImage = vi.fn(async (_options: { url: string; ttl: number }) => "");

  const manager = new ServerRosterManager({
    serverList: serverList as never,
    preflight,
    imageCache: { getImage } as never,
    forgetCredentials,
    checkForUpdates,
  });

  return { manager, serverList, preflight, forgetCredentials, checkForUpdates, getImage };
}

describe("loading the list", () => {
  it("draws every saved server before any of them is checked", async () => {
    const { manager, preflight } = build({ servers: ["https://a", "https://b"] });
    const count = await manager.load();

    expect(count).toBe(2);
    expect(get(manager.entries).map((e) => e.status)).toEqual(["checking", "checking"]);
    expect(preflight).not.toHaveBeenCalled();
  });

  // Four pending blocks are what the strip draws before anything has run.
  it("gives each plate a full set of unstarted checks", async () => {
    const { manager } = build({ servers: ["https://a"] });
    await manager.load();
    const steps = get(manager.entries)[0].steps;
    expect(steps).toHaveLength(4);
    expect(steps.every((step) => step.state === "pending")).toBe(true);
  });

  it("shows the host without its scheme, which is what a person recognises", async () => {
    const { manager } = build({ servers: ["https://bvc.example.com/"] });
    await manager.load();
    expect(get(manager.entries)[0].host).toBe("bvc.example.com");
  });

  it("carries each preflight's conclusion onto its own plate", async () => {
    const { manager } = build({
      servers: ["https://a", "https://b"],
      outcomes: { "https://a": outcome("connect"), "https://b": outcome("udp_blocked") },
    });
    await manager.load();
    await manager.sweep();
    expect(get(manager.entries).map((e) => e.status)).toEqual(["connect", "udp_blocked"]);
  });

  /**
   * The reason the sweep is separate from the draw. A dead host takes as long as its timeout,
   * and holding every other plate behind it would make one broken server look like a broken
   * app.
   */
  it("preflights servers concurrently rather than one after another", async () => {
    let running = 0;
    let peak = 0;
    const preflight = vi.fn(async () => {
      running++;
      peak = Math.max(peak, running);
      await Promise.resolve();
      running--;
      return outcome("connect");
    });

    const manager = new ServerRosterManager({
      serverList: {
        getServerList: async () => [
          { server: "https://a", player: "p" },
          { server: "https://b", player: "p" },
          { server: "https://c", player: "p" },
        ],
      } as never,
      preflight,
      imageCache: { getImage: async () => "" } as never,
      forgetCredentials: async () => {},
      checkForUpdates: async () => null,
    });

    await manager.load();
    await manager.sweep();
    expect(peak).toBe(3);
  });

  // A plate resolves as its own checks land, which needs every intermediate list published.
  it("publishes step changes onto the plate as they arrive", async () => {
    const { manager } = build({ servers: ["https://a"] });
    const preflight = vi.fn(async (_server: string, observe: (steps: never) => void) => {
      const steps = PreflightRunner.pending();
      steps[0] = { ...steps[0], state: "ok", note: "signed in as Alaydriem", ms: 12 };
      observe(steps as never);
      return outcome("connect");
    });

    const live = new ServerRosterManager({
      serverList: { getServerList: async () => [{ server: "https://a", player: "p" }] } as never,
      preflight,
      imageCache: { getImage: async () => "" } as never,
      forgetCredentials: async () => {},
      checkForUpdates: async () => null,
    });
    await live.load();
    await live.sweep();

    expect(get(live.entries)[0].steps[0].note).toBe("signed in as Alaydriem");
    expect(manager).toBeDefined();
  });
});

describe("operator art", () => {
  /**
   * Both assets are absent far more often than not, and the derived glyph is the case that
   * always works — so art must never be able to hold a plate back.
   */
  it("fetches both assets without blocking the list", async () => {
    const { manager, getImage } = build({ servers: ["https://a"] });
    await manager.load();
    await Promise.resolve();
    expect(getImage).toHaveBeenCalledTimes(2);
    const urls = getImage.mock.calls.map(([options]) => options.url);
    expect(urls).toContain("https://a/assets/avatar.png");
    expect(urls).toContain("https://a/assets/canvas.png");
  });

  it("leaves the plate on its derived identity when neither asset exists", async () => {
    const { manager } = build({ servers: ["https://a"] });
    await manager.load();
    expect(get(manager.entries)[0].avatarUrl).toBe("");
    expect(get(manager.entries)[0].canvasUrl).toBe("");
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
      outcomes: { "https://a": outcome("reauth") },
    });
    await manager.load();
    await manager.sweep();

    expect(await manager.choose("https://a")).toEqual({
      kind: "navigate",
      href: "/login?reauth=true&server=https://a",
    });
  });

  /**
   * Voice is the product, so there is nothing to connect to without a UDP path. Rechecking is
   * the only thing worth offering, and it must happen rather than navigate.
   */
  it("rechecks a voice-blocked server instead of connecting to it", async () => {
    const { manager, preflight } = build({
      servers: ["https://a"],
      outcomes: { "https://a": outcome("udp_blocked") },
    });
    await manager.load();
    await manager.sweep();
    preflight.mockClear();

    expect(await manager.choose("https://a")).toEqual({ kind: "none" });
    expect(preflight).toHaveBeenCalledOnce();
  });

  it("rechecks a server that was not answering", async () => {
    const { manager, preflight } = build({
      servers: ["https://a"],
      outcomes: { "https://a": outcome("unreachable") },
    });
    await manager.load();
    await manager.sweep();
    preflight.mockClear();

    expect(await manager.choose("https://a")).toEqual({ kind: "none" });
    expect(preflight).toHaveBeenCalledOnce();
  });

  it("goes nowhere while checks are still running", async () => {
    const { manager } = build({ servers: ["https://a"] });
    await manager.load();
    expect(await manager.choose("https://a")).toEqual({ kind: "none" });
  });

  it("goes to the update screen when this build is behind and an update exists", async () => {
    const { manager } = build({
      servers: ["https://a"],
      outcomes: { "https://a": outcome("version_mismatch", { clientTooOld: true }) },
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
   * An update screen with no update to install is a dead end. Saying so on the plate is the
   * honest answer, and it is also the true one: the server is ahead of every build.
   */
  it("says so on the plate when this build is behind and no update exists", async () => {
    const { manager } = build({
      servers: ["https://a"],
      outcomes: { "https://a": outcome("version_mismatch", { clientTooOld: true }) },
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
      outcomes: { "https://a": outcome("version_mismatch", { clientTooOld: false }) },
    });
    await manager.load();
    await manager.sweep();

    expect(await manager.choose("https://a")).toEqual({ kind: "none" });
    expect(checkForUpdates).not.toHaveBeenCalled();
  });
});

describe("rechecking one server", () => {
  it("returns that plate to checking with a fresh set of steps", async () => {
    const { manager } = build({
      servers: ["https://a", "https://b"],
      outcomes: { "https://a": outcome("udp_blocked") },
    });
    await manager.load();
    await manager.sweep();

    const pending = manager.recheck("https://a");
    expect(get(manager.entries)[0].status).toBe("checking");
    await pending;
  });

  // A note is about the last attempt, not about the server, so a new attempt clears it.
  it("clears a note left by a previous attempt", async () => {
    const { manager } = build({
      servers: ["https://a"],
      outcomes: { "https://a": outcome("version_mismatch", { clientTooOld: true }) },
      update: null,
    });
    await manager.load();
    await manager.sweep();
    await manager.choose("https://a");
    expect(get(manager.entries)[0].note).toBeTruthy();

    await manager.recheck("https://a");
    expect(get(manager.entries)[0].note).toBeUndefined();
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
   * Credentials that outlive their plate are invisible and stay on the device, so a keyring
   * that refuses must not stop the entry being removed.
   */
  it("still removes the entry when the credentials cannot be cleared", async () => {
    const { serverList } = build({ servers: ["https://a", "https://b"] });
    const failing = new ServerRosterManager({
      serverList: serverList as never,
      preflight: async () => outcome("connect"),
      imageCache: { getImage: async () => "" } as never,
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
  function row(status: PreflightOutcome["status"], server = "https://a") {
    return {
      server,
      host: server,
      player: "p",
      game: "minecraft",
      status,
      steps: PreflightRunner.pending(),
      rtt: 0,
      slow: false,
      quicPort: 443,
      serverVersion: "",
      clientVersion: "",
      clientTooOld: false,
      avatarUrl: "",
      canvasUrl: "",
    };
  }

  it("skips the list when the only saved server passed its preflight", () => {
    expect(ServerRosterManager.autoJoin([row("connect")])).toBe("https://a");
  });

  // Everything the list would have said about a single broken server is worth saying.
  it("shows the list when the only saved server failed anything", () => {
    for (const status of [
      "reauth",
      "udp_blocked",
      "unreachable",
      "version_mismatch",
    ] as const) {
      expect(ServerRosterManager.autoJoin([row(status)])).toBeNull();
    }
  });

  it("never skips a choice between two servers", () => {
    expect(ServerRosterManager.autoJoin([row("connect"), row("connect", "https://b")])).toBeNull();
  });
});

describe("rechecking everything", () => {
  let flag: boolean[] = [];
  beforeEach(() => (flag = []));

  it("reports it is working while every server is re-checked", async () => {
    const { manager } = build({ servers: ["https://a"] });
    manager.isRefreshing.subscribe((v) => flag.push(v));

    await manager.refreshAll();

    expect(flag).toEqual([false, true, false]);
  });
});
