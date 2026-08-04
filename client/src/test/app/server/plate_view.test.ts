import { describe, expect, it } from "vitest";
import { PlateView } from "../../../js/app/server/PlateView";
import { PreflightRunner } from "../../../js/app/server/preflight/PreflightRunner";
import type { RosterStatus } from "../../../js/app/server/RosterStatus";
import type { ServerRosterEntry } from "../../../js/app/server/ServerRosterEntry";

function entry(status: RosterStatus, overrides: Partial<ServerRosterEntry> = {}) {
  return {
    server: "https://bvc.example.com",
    host: "bvc.example.com",
    player: "Alaydriem",
    game: "minecraft",
    status,
    steps: PreflightRunner.pending(),
    rtt: 24,
    slow: false,
    quicPort: 443,
    serverVersion: "2.1.0",
    clientVersion: "2.1.0",
    clientTooOld: false,
    avatarUrl: "",
    canvasUrl: "",
    ...overrides,
  } as ServerRosterEntry;
}

describe("PlateView.of", () => {
  it("offers the connect on a server that passed everything", () => {
    const view = PlateView.of(entry("connect"));
    expect(view.chip).toBe("Ready");
    expect(view.severity).toBe("ok");
    expect(view.kind).toBe("connect");
  });

  // A round trip worth noticing is still connectable, so it warns rather than blocks.
  it("says so when a working server is slow", () => {
    const view = PlateView.of(entry("connect", { slow: true, rtt: 186 }));
    expect(view.chip).toBe("Ready · slow");
    expect(view.severity).toBe("warn");
    expect(view.kind).toBe("connect");
  });

  /**
   * Voice is the product. There is nothing worth connecting to without a UDP path and the
   * connection would fail anyway, so a "connect anyway" would only sell a failure as a
   * choice.
   */
  it("never offers a connect on a server with no UDP path", () => {
    const view = PlateView.of(entry("udp_blocked"));
    expect(view.chip).toBe("Voice blocked");
    expect(view.kind).toBe("recheck");
  });

  it("offers a recheck rather than a sign-in for a server that is not answering", () => {
    const view = PlateView.of(entry("unreachable"));
    expect(view.kind).toBe("recheck");
    expect(view.action).not.toMatch(/sign in/i);
  });

  it("offers a sign-in when the sign-in is what is missing", () => {
    expect(PlateView.of(entry("reauth")).kind).toBe("signin");
  });

  it("offers the update when this build is the older one", () => {
    const view = PlateView.of(entry("version_mismatch", { clientTooOld: true }));
    expect(view.chip).toBe("Update needed");
    expect(view.action).toBe("Update");
  });

  /**
   * Pointing "Update needed" at somebody whose client is the newer of the two sends them
   * looking for an update that does not exist and would not help.
   */
  it("does not ask for an update when the server is the older one", () => {
    const view = PlateView.of(entry("version_mismatch", { clientTooOld: false }));
    expect(view.chip).toBe("Server is older");
    expect(view.chip).not.toMatch(/update/i);
    expect(view.kind).toBe("blocked");
  });

  it("promises nothing while checks are still running", () => {
    const view = PlateView.of(entry("checking"));
    expect(view.severity).toBe("muted");
    expect(view.kind).toBe("blocked");
  });

  // A plate always says something. A blank chip on a status this mapping forgot would read
  // as a plate that had finished and found nothing wrong.
  it("names every status a preflight can produce", () => {
    const all: RosterStatus[] = [
      "checking",
      "connect",
      "reauth",
      "version_mismatch",
      "udp_blocked",
      "unreachable",
    ];
    for (const status of all) {
      const view = PlateView.of(entry(status));
      expect(view.chip.length).toBeGreaterThan(0);
      expect(view.action.length).toBeGreaterThan(0);
    }
  });
});

describe("PlateView.isJoinable", () => {
  it("is only true for a server that passed all four checks", () => {
    expect(PlateView.isJoinable(entry("connect"))).toBe(true);
    for (const status of [
      "checking",
      "reauth",
      "udp_blocked",
      "unreachable",
      "version_mismatch",
    ] as RosterStatus[]) {
      expect(PlateView.isJoinable(entry(status))).toBe(false);
    }
  });
});

describe("PlateView.tally", () => {
  it("counts servers in the same state together", () => {
    const tally = PlateView.tally([
      entry("connect", { server: "https://a" }),
      entry("connect", { server: "https://b" }),
      entry("udp_blocked", { server: "https://c" }),
    ]);
    expect(tally).toEqual([
      { label: "ready", count: 2, severity: "ok" },
      { label: "voice blocked", count: 1, severity: "bad" },
    ]);
  });

  // Slow is a different answer from ready, and a bar that merged them would report a list
  // as entirely fine when half of it is not.
  it("separates a slow server from a ready one", () => {
    const tally = PlateView.tally([
      entry("connect", { server: "https://a" }),
      entry("connect", { server: "https://b", slow: true }),
    ]);
    expect(tally.map((item) => item.label)).toEqual(["ready", "slow"]);
  });

  it("has nothing to report for an empty list", () => {
    expect(PlateView.tally([])).toEqual([]);
  });
});
