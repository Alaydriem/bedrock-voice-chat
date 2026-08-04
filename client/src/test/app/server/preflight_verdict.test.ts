import { describe, expect, it } from "vitest";
import { PreflightVerdict } from "../../../js/app/server/preflight/PreflightVerdict";
import { PREFLIGHT_STEPS, type PreflightStepName } from "../../../js/app/server/preflight/PreflightStepName";
import type { PreflightStepState } from "../../../js/app/server/preflight/PreflightStepState";
import type { RosterStatus } from "../../../js/app/server/RosterStatus";
import type { ServerRosterEntry } from "../../../js/app/server/ServerRosterEntry";

/** A finished preflight where one named check failed and everything before it passed. */
function failedAt(
  name: PreflightStepName | null,
  status: RosterStatus,
  overrides: Partial<ServerRosterEntry> = {},
) {
  const index = name === null ? PREFLIGHT_STEPS.length : PREFLIGHT_STEPS.indexOf(name);
  const steps = PREFLIGHT_STEPS.map((stepName, i) => ({
    name: stepName,
    state: (i < index ? "ok" : i === index ? "bad" : "skipped") as PreflightStepState,
    note: "",
    ms: 10,
  }));

  return {
    server: "https://bvc.example.com",
    host: "bvc.example.com",
    player: "Alaydriem",
    game: "minecraft",
    status,
    steps,
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

describe("PreflightVerdict.of", () => {
  /**
   * First failing check wins. A summary that averaged four results into "mostly fine" would
   * leave somebody with a problem to find the one line that is not.
   */
  it("leads with the check that failed, not the ones that passed", () => {
    const verdict = PreflightVerdict.of(failedAt("QUIC path", "udp_blocked"));
    expect(verdict.severity).toBe("bad");
    expect(verdict.sentence).toMatch(/UDP 443/);
  });

  it("tells someone with no sign-in to sign in again", () => {
    const verdict = PreflightVerdict.of(failedAt("Credentials", "reauth"));
    expect(verdict.sentence).toMatch(/sign in again/i);
  });

  // The same check fails for both, and the two answers send someone to different places.
  it("separates a refused sign-in from a server that is not there", () => {
    const refused = PreflightVerdict.of(failedAt("Handshake", "reauth"));
    const absent = PreflightVerdict.of(failedAt("Handshake", "unreachable"));
    expect(refused.sentence).toMatch(/sign in again/i);
    expect(absent.sentence).toMatch(/ask whoever runs it/i);
    expect(absent.sentence).not.toMatch(/sign in/i);
  });

  it("names both protocols and points the update at the right side", () => {
    const behind = PreflightVerdict.of(
      failedAt("Protocol", "version_mismatch", {
        clientTooOld: true,
        serverVersion: "2.2.0",
        clientVersion: "2.1.0",
      }),
    );
    expect(behind.sentence).toMatch(/2\.2\.0/);
    expect(behind.sentence).toMatch(/2\.1\.0/);
    expect(behind.sentence).toMatch(/Update the client/i);
  });

  it("does not tell someone to update when their client is the newer one", () => {
    const ahead = PreflightVerdict.of(
      failedAt("Protocol", "version_mismatch", {
        clientTooOld: false,
        serverVersion: "2.0.0",
        clientVersion: "2.1.0",
      }),
    );
    expect(ahead.sentence).toMatch(/whoever runs it/i);
    expect(ahead.sentence).not.toMatch(/update the client/i);
  });

  // A warning is not a failure: the server works and the delay is worth knowing about.
  it("reports a slow link as a warning with the measurement in it", () => {
    const slow = failedAt(null, "connect", { slow: true, rtt: 186 });
    const warned = {
      ...slow,
      steps: slow.steps.map((step) =>
        step.name === "Handshake" ? { ...step, state: "warn" as PreflightStepState } : step,
      ),
    } as ServerRosterEntry;
    const verdict = PreflightVerdict.of(warned);
    expect(verdict.severity).toBe("warn");
    expect(verdict.sentence).toMatch(/186 ms/);
  });

  it("says everything is fine when nothing failed or warned", () => {
    const verdict = PreflightVerdict.of(failedAt(null, "connect"));
    expect(verdict.severity).toBe("ok");
    expect(verdict.sentence).toBe("Everything looks fine.");
  });

  it("does not pass judgement while checks are still running", () => {
    const verdict = PreflightVerdict.of(failedAt(null, "checking"));
    expect(verdict.sentence).toBe("Checking…");
  });
});
