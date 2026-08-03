import { describe, expect, it } from "vitest";
import AddressResolver from "../../../js/app/login/AddressResolver";
import type { ServerReachability } from "../../../js/bindings/ServerReachability";
import type { ReachabilityVerdict } from "../../../js/bindings/ReachabilityVerdict";

function report(
  verdict: ReachabilityVerdict,
  bestRtt: number | null,
  quicPorts: number[] = [443],
): ServerReachability {
  return {
    host: "bvc.example.com",
    quic: quicPorts.map((port) => ({
      addr: `10.0.0.1:${port}`,
      family: "Ipv4",
      port,
      outcome:
        bestRtt === null
          ? { state: "silent" }
          : { state: "answered", via: "VersionNegotiation", rtt_micros: bestRtt },
      certificate: null,
    })),
    https: [],
    preference: "PreferIpv4",
    verdict,
    best_rtt_micros: bestRtt,
  } as unknown as ServerReachability;
}

describe("AddressResolver.verdictFor", () => {
  it("reports the measured round trip in milliseconds", () => {
    const verdict = AddressResolver.verdictFor(report("Ready", 41_000));
    expect(verdict.state).toBe("ok");
    expect(verdict.ring).toBe("lock");
    expect(verdict.line).toMatch(/41 ms/);
  });

  it("rounds a fractional timing to whole milliseconds", () => {
    expect(AddressResolver.verdictFor(report("Ready", 1_600)).line).toMatch(/2 ms/);
  });

  // Anything other than 443 is a fallback, and saying so is the difference between
  // a working server and a mystery.
  it("names a non-standard QUIC port", () => {
    expect(AddressResolver.verdictFor(report("Ready", 20_000, [8443])).line).toMatch(/8443/);
  });

  it("does not clutter the line when QUIC answered on 443", () => {
    expect(AddressResolver.verdictFor(report("Ready", 20_000, [443])).line).not.toMatch(/443/);
  });

  // The specific, common failure: a network that permits HTTPS and drops UDP. It is
  // not "nothing at that address", and it sends the player somewhere different.
  it("distinguishes a reachable host with no voice path", () => {
    const verdict = AddressResolver.verdictFor(report("VoiceBlocked", null));
    expect(verdict.state).toBe("bad");
    expect(verdict.ring).toBe("empty");
    expect(verdict.line).toMatch(/voice/i);
  });

  it("reports nothing answering at all", () => {
    const verdict = AddressResolver.verdictFor(report("Unreachable", null));
    expect(verdict.state).toBe("bad");
    expect(verdict.line).toMatch(/Nothing at that address/);
  });

  // No route is the local stack's answer, and it earns its own message.
  it("distinguishes having no route from silence", () => {
    const verdict = AddressResolver.verdictFor(report("NoRoute", null));
    expect(verdict.state).toBe("bad");
    expect(verdict.line).toMatch(/no route/i);
  });

  // A probe that could not run is an unknown, not a failure. Sign-in stays enabled
  // in every one of these states; see the SignInScreen tests.
  it("treats a missing report as unknown rather than failed", () => {
    const verdict = AddressResolver.verdictFor(null);
    expect(verdict.state).toBe("editing");
    expect(verdict.ring).toBe("empty");
  });

  // The verdict is derived in Rust so login and the server selector cannot
  // disagree. If this ever falls through to a default, the mapping has drifted.
  it("maps every verdict the Rust side can produce", () => {
    const all: ReachabilityVerdict[] = ["Ready", "VoiceBlocked", "Unreachable", "NoRoute"];
    for (const v of all) {
      const mapped = AddressResolver.verdictFor(report(v, v === "Ready" ? 9_000 : null));
      expect(mapped.line.length).toBeGreaterThan(0);
      expect(mapped.caption.length).toBeGreaterThan(0);
    }
  });
});

describe("AddressResolver.verdictForIncompatible", () => {
  // "Nothing at that address" sends someone to check what they typed. This sends
  // them to an update. Collapsing the two would send them to the wrong place.
  it("names both versions rather than saying the server is unreachable", () => {
    const verdict = AddressResolver.verdictForIncompatible({
      server_version: "2.2.0",
      client_version: "2.1.0",
      compatible: false,
      client_too_old: true,
    });
    expect(verdict.state).toBe("bad");
    expect(verdict.line).toMatch(/2\.2\.0/);
    expect(verdict.line).toMatch(/2\.1\.0/);
    expect(verdict.line).not.toMatch(/nothing at that address/i);
  });

  it("tells a behind client that an update is the fix", () => {
    const verdict = AddressResolver.verdictForIncompatible({
      server_version: "2.2.0",
      client_version: "2.1.0",
      compatible: false,
      client_too_old: true,
    });
    expect(verdict.line).toMatch(/newer app/i);
  });

  // The other direction is not fixable by updating, so it must not say so.
  it("does not promise an update will help an ahead client", () => {
    const verdict = AddressResolver.verdictForIncompatible({
      server_version: "2.0.0",
      client_version: "2.1.0",
      compatible: false,
      client_too_old: false,
    });
    expect(verdict.line).toMatch(/older protocol/i);
    expect(verdict.line).not.toMatch(/newer app/i);
  });

  it("puts the server's protocol on the ring caption", () => {
    const verdict = AddressResolver.verdictForIncompatible({
      server_version: "2.2.0",
      client_version: "2.1.0",
      compatible: false,
      client_too_old: true,
    });
    expect(verdict.caption).toBe("PROTOCOL 2.2.0");
  });
});

describe("AddressResolver.HOSTNAME", () => {
  it("accepts something that could be a host", () => {
    expect(AddressResolver.HOSTNAME.test("bvc.example.com")).toBe(true);
  });

  // Only shapes that cannot become a hostname are rejected. A short TLD is not one
  // of those, so "bvc.io" has to pass; spending a probe on a plausible address is
  // the intended cost.
  it("rejects a value that cannot be a hostname, so no probe is spent on it", () => {
    expect(AddressResolver.HOSTNAME.test("bvc")).toBe(false);
    expect(AddressResolver.HOSTNAME.test("bvc.")).toBe(false);
    expect(AddressResolver.HOSTNAME.test("bvc.e")).toBe(false);
    expect(AddressResolver.HOSTNAME.test("")).toBe(false);
  });

  it("accepts a short top-level domain", () => {
    expect(AddressResolver.HOSTNAME.test("bvc.io")).toBe(true);
  });
});
