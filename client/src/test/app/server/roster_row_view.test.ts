import { describe, expect, it } from "vitest";
import { RosterRowView } from "../../../js/app/server/RosterRowView";
import type { RosterStatus } from "../../../js/app/server/RosterStatus";
import type { ServerRosterEntry } from "../../../js/app/server/ServerRosterEntry";

function entry(status: RosterStatus, overrides: Partial<ServerRosterEntry> = {}) {
  return {
    server: "https://bvc.example.com",
    host: "bvc.example.com",
    player: "Alaydriem",
    game: "minecraft",
    status,
    serverVersion: "",
    clientVersion: "",
    clientTooOld: false,
    isCurrent: false,
    ...overrides,
  } as ServerRosterEntry;
}

describe("RosterRowView.of", () => {
  it("offers the join on a server that is ready", () => {
    const view = RosterRowView.of(entry("connect"));
    expect(view.severity).toBe("ok");
    expect(view.action).toBe("Join");
  });

  /**
   * The distinction the extra status exists for. A lapsed sign-in is fixed by signing in;
   * a server that is not answering is not, and offering a sign-in there sends someone to a
   * Microsoft prompt that cannot succeed.
   */
  it("does not offer a sign-in for a server that is not answering", () => {
    const view = RosterRowView.of(entry("unreachable"));
    expect(view.severity).toBe("bad");
    expect(view.action).not.toMatch(/sign in/i);
  });

  it("offers a sign-in for a lapsed sign-in", () => {
    expect(RosterRowView.of(entry("reauth")).action).toMatch(/sign in/i);
  });

  it("offers a sign-in for a server with no credentials saved", () => {
    expect(RosterRowView.of(entry("missing")).action).toMatch(/sign in/i);
  });

  // The ring is reachability and the chip is what to do about it, so a lapsed sign-in
  // reads as found while still warning. Collapsing the two loses that.
  it("shows a reachable server as found even when its sign-in has lapsed", () => {
    const view = RosterRowView.of(entry("reauth"));
    expect(view.ring).toBe("lock");
    expect(view.severity).toBe("warn");
  });

  it("shows nothing found for a server that is not answering", () => {
    expect(RosterRowView.of(entry("unreachable")).ring).toBe("empty");
  });

  it("offers an update when this build is the older one", () => {
    const view = RosterRowView.of(
      entry("version_mismatch", { clientTooOld: true, serverVersion: "2.2.0" }),
    );
    expect(view.action).toBe("Update");
    expect(view.blocked).toBe("");
  });

  /**
   * The other direction is not fixable from here. An action would be a promise this app
   * cannot keep, so the row explains instead and names both versions.
   */
  it("explains rather than acts when the server is the older one", () => {
    const view = RosterRowView.of(
      entry("version_mismatch", {
        clientTooOld: false,
        serverVersion: "2.0.0",
        clientVersion: "2.1.0",
      }),
    );
    expect(view.action).toBe("");
    expect(view.blocked).toMatch(/2\.0\.0/);
    expect(view.blocked).toMatch(/2\.1\.0/);
  });

  it("offers nothing while a check is still running", () => {
    expect(RosterRowView.of(entry("checking")).action).toBe("");
  });

  // A row always says something. "Checking" forever is not an answer, and neither is a
  // blank chip on a status this mapping forgot.
  it("names every status a check can produce", () => {
    const all: RosterStatus[] = [
      "checking",
      "connect",
      "reauth",
      "missing",
      "version_mismatch",
      "unreachable",
    ];
    for (const status of all) {
      const view = RosterRowView.of(entry(status));
      expect(view.status.length).toBeGreaterThan(0);
      expect(view.caption.length).toBeGreaterThan(0);
    }
  });
});

describe("RosterRowView.isJoinable", () => {
  it("is only true for a server that answered over its own credentials", () => {
    expect(RosterRowView.isJoinable(entry("connect"))).toBe(true);
    for (const status of ["checking", "reauth", "missing", "unreachable"] as RosterStatus[]) {
      expect(RosterRowView.isJoinable(entry(status))).toBe(false);
    }
  });
});

describe("RosterRowView.resting", () => {
  it("rests on the server that can be joined rather than the first one stored", () => {
    const entries = [entry("unreachable"), entry("connect", { server: "https://b", host: "b" })];
    expect(RosterRowView.resting(entries)?.host).toBe("b");
  });

  it("prefers the server already in use when more than one is ready", () => {
    const entries = [
      entry("connect", { server: "https://a", host: "a" }),
      entry("connect", { server: "https://b", host: "b", isCurrent: true }),
    ];
    expect(RosterRowView.resting(entries)?.host).toBe("b");
  });

  // A list of entirely broken servers still has to read as something.
  it("falls back to the first row when none can be joined", () => {
    const entries = [entry("unreachable", { host: "a" }), entry("reauth", { host: "b" })];
    expect(RosterRowView.resting(entries)?.host).toBe("a");
  });

  it("has nothing to read when the list is empty", () => {
    expect(RosterRowView.resting([])).toBeNull();
  });
});
