import { describe, expect, it, vi } from "vitest";
import { invokeCalls, mockInvoke } from "../../tauri";
import { PreflightRunner } from "../../../js/app/server/preflight/PreflightRunner";
import type { PreflightStep } from "../../../js/app/server/preflight/PreflightStep";

/** The unauthenticated config read, which decides why a handshake failed. */
let answering = true;
vi.mock("@tauri-apps/plugin-http", () => ({
  fetch: vi.fn(async () => {
    if (!answering) throw new Error("connection refused");
    return { status: 200, json: async () => ({ protocol_version: "2.1.0", quic_ports: [443] }) };
  }),
}));

const CREDENTIALS = {
  gamertag: "Alaydriem",
  certificate_ca: "ca",
  certificate: "cert",
  certificate_key: "key",
};

const SERVER = "https://bvc.example.com";

function config(overrides: Record<string, unknown> = {}) {
  return {
    config: { protocol_version: "2.1.0", quic_port: 443, quic_ports: [443] },
    client_version: "2.1.0",
    compatible: true,
    client_too_old: false,
    ...overrides,
  };
}

function reachability(verdict: string, port = 443, rtt: number | null = 24_000) {
  return {
    host: "bvc.example.com",
    quic: [
      {
        addr: `10.0.0.1:${port}`,
        family: "Ipv4",
        port,
        outcome: rtt === null ? { state: "silent" } : { state: "answered", rtt_micros: rtt },
        certificate: null,
      },
    ],
    https: [],
    preference: "PreferIpv4",
    verdict,
    best_rtt_micros: rtt,
  };
}

function ipc(overrides: Record<string, (args: never) => unknown> = {}) {
  answering = true;
  mockInvoke({
    get_credentials: () => CREDENTIALS,
    is_certificate_expired: () => false,
    api_pool_client: () => null,
    api_get_config: () => config(),
    probe_server: () => reachability("Ready"),
    ...overrides,
  });
}

/** Runs a preflight and keeps every intermediate step list the observer saw. */
async function run(): Promise<{
  outcome: Awaited<ReturnType<PreflightRunner["run"]>>;
  frames: PreflightStep[][];
  steps: PreflightStep[];
}> {
  const frames: PreflightStep[][] = [];
  const runner = new PreflightRunner((steps) => frames.push([...steps]));
  const outcome = await runner.run(SERVER);
  return { outcome, frames, steps: frames[frames.length - 1] };
}

describe("a server that passes everything", () => {
  it("concludes it can be connected to", async () => {
    ipc();
    const { outcome, steps } = await run();
    expect(outcome.status).toBe("connect");
    expect(steps.map((step) => step.state)).toEqual(["ok", "ok", "ok", "ok"]);
  });

  it("names the account rather than the certificate behind it", async () => {
    ipc();
    const { steps } = await run();
    expect(steps[0].note).toBe("signed in as Alaydriem");
    expect(steps[0].note).not.toMatch(/certificate|expir|mtls/i);
  });

  it("reports the UDP path as open on the port that answered", async () => {
    ipc();
    const { steps } = await run();
    expect(steps[3].note).toMatch(/udp\/443 open/);
  });

  // Anything other than 443 is a fallback, and saying so is the difference between a
  // working server and a mystery.
  it("names a non-standard QUIC port as a fallback", async () => {
    ipc({
      api_get_config: () => config({ config: { protocol_version: "2.1.0", quic_port: 8443, quic_ports: [8443] } }),
      probe_server: () => reachability("Ready", 8443),
    });
    const { steps, outcome } = await run();
    expect(steps[3].note).toMatch(/fallback port/);
    expect(outcome.quicPort).toBe(8443);
  });

  /**
   * The sweep must never claim the session. `current_server` decides which server `logout`
   * clears and which one commands with no explicit endpoint act on, so checking every saved
   * server would otherwise leave it pointing at whichever check finished last.
   */
  it("makes the server callable without claiming it as the current one", async () => {
    ipc();
    await run();
    const commands = invokeCalls().map((call) => call.cmd);
    expect(commands).toContain("api_pool_client");
    expect(commands).not.toContain("api_initialize_client");
  });

  // Rotating a certificate is a write, and looking at a list of servers is not a reason to
  // do it for every server on it.
  it("does not rotate credentials while checking", async () => {
    ipc();
    await run();
    expect(invokeCalls().map((call) => call.cmd)).not.toContain("refresh_server_state");
  });
});

describe("a failing check", () => {
  /**
   * The whole point of the sequence. A reader left at "pending" is waiting for a result that
   * is not coming, so the checks that never ran say so.
   */
  it("stops the sequence and marks the rest as never run", async () => {
    ipc({ is_certificate_expired: () => true });
    const { steps } = await run();
    expect(steps.map((step) => step.state)).toEqual(["bad", "skipped", "skipped", "skipped"]);
    expect(steps[1].note).toBe("not run");
  });

  it("does not probe a server it could not sign in to", async () => {
    ipc({ is_certificate_expired: () => true });
    await run();
    expect(invokeCalls().map((call) => call.cmd)).not.toContain("probe_server");
  });

  it("reports a missing sign-in the same way as an expired one", async () => {
    ipc({
      get_credentials: () => {
        throw new Error("no entry");
      },
    });
    const { outcome, steps } = await run();
    expect(outcome.status).toBe("reauth");
    expect(steps[0].note).toBe("no valid sign-in for this server");
  });
});

describe("a handshake that fails", () => {
  /**
   * The failure this structure exists to separate. A server that is down and a certificate
   * the server refused both fail here, and they lead to different places: one to a sign-in,
   * one to whoever runs the server.
   */
  it("reads as a lapsed sign-in when the server itself is up", async () => {
    ipc({
      api_get_config: () => {
        throw new Error("certificate rejected");
      },
    });
    const { outcome, steps } = await run();
    expect(outcome.status).toBe("reauth");
    expect(steps[1].note).toMatch(/refused/);
  });

  it("reads as unreachable when nothing answers at all", async () => {
    ipc({
      api_get_config: () => {
        throw new Error("connection refused");
      },
    });
    answering = false;
    const { outcome, steps } = await run();
    expect(outcome.status).toBe("unreachable");
    expect(steps[1].note).toBe("no response");
  });
});

describe("protocol", () => {
  it("names both versions and which side is behind", async () => {
    ipc({
      api_get_config: () =>
        config({
          config: { protocol_version: "2.2.0", quic_port: 443, quic_ports: [443] },
          compatible: false,
          client_too_old: true,
        }),
    });
    const { outcome, steps } = await run();
    expect(outcome.status).toBe("version_mismatch");
    expect(outcome.clientTooOld).toBe(true);
    expect(steps[2].note).toMatch(/client 2\.1\.0/);
    expect(steps[2].note).toMatch(/server 2\.2\.0/);
    expect(steps[2].note).toMatch(/client is too old/);
  });

  // The other direction is not fixable by updating, so it must not say so.
  it("does not blame the client when the server is the older one", async () => {
    ipc({
      api_get_config: () =>
        config({
          config: { protocol_version: "2.0.0", quic_port: 443, quic_ports: [443] },
          compatible: false,
          client_too_old: false,
        }),
    });
    const { steps } = await run();
    expect(steps[2].note).toMatch(/server is too old/);
  });

  it("carries the versions out even though the sequence stopped", async () => {
    ipc({
      api_get_config: () =>
        config({
          config: { protocol_version: "2.2.0", quic_port: 443, quic_ports: [443] },
          compatible: false,
          client_too_old: true,
        }),
    });
    const { outcome } = await run();
    expect(outcome.serverVersion).toBe("2.2.0");
    expect(outcome.clientVersion).toBe("2.1.0");
  });
});

describe("the QUIC path", () => {
  /**
   * The check that earns its place. Every check above it ran over TCP 443, so a network that
   * permits HTTPS and drops UDP passes all three and then cannot carry one audio frame.
   */
  it("blocks a server that answered over TCP but not UDP", async () => {
    ipc({ probe_server: () => reachability("VoiceBlocked", 443, null) });
    const { outcome, steps } = await run();
    expect(outcome.status).toBe("udp_blocked");
    expect(steps[1].state).toBe("ok");
    expect(steps[3].note).toMatch(/unreachable/);
  });

  // No route is the local stack's answer and it earns its own wording: nothing about the
  // server or the firewall is the problem.
  it("distinguishes having no route from a silent server", async () => {
    ipc({ probe_server: () => reachability("NoRoute", 443, null) });
    const { steps } = await run();
    expect(steps[3].note).toMatch(/no route/i);
  });

  it("blocks rather than guesses when the probe itself fails", async () => {
    ipc({
      probe_server: () => {
        throw new Error("probe unavailable");
      },
    });
    const { outcome } = await run();
    expect(outcome.status).toBe("udp_blocked");
  });
});

describe("the observer", () => {
  /**
   * The plates resolve as their own checks land, which is only possible if every transition
   * is published rather than just the final list.
   */
  it("publishes each step as it starts and again as it finishes", async () => {
    ipc();
    const { frames } = await run();
    const running = frames.filter((steps) => steps.some((step) => step.state === "running"));
    expect(running.length).toBe(4);
    expect(frames.length).toBeGreaterThan(8);
  });

  it("times every step it ran", async () => {
    ipc();
    const { steps } = await run();
    for (const step of steps) expect(step.ms).toBeGreaterThan(0);
  });
});
