import { describe, expect, it, vi } from "vitest";
import { render, screen, within } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import PreflightPanel from "../../components/server/PreflightPanel.svelte";
import { PREFLIGHT_STEPS } from "../../js/app/server/preflight/PreflightStepName";
import type { PreflightStepState } from "../../js/app/server/preflight/PreflightStepState";
import type { RosterStatus } from "../../js/app/server/RosterStatus";
import type { ServerRosterEntry } from "../../js/app/server/ServerRosterEntry";

function entry(status: RosterStatus, overrides: Partial<ServerRosterEntry> = {}) {
  return {
    server: "https://bvc.alaydriem.com",
    host: "bvc.alaydriem.com",
    player: "Alaydriem",
    game: "minecraft",
    status,
    steps: PREFLIGHT_STEPS.map((name) => ({
      name,
      state: "ok" as PreflightStepState,
      note: `${name} note`,
      ms: 25,
    })),
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

function props(overrides: Record<string, unknown> = {}) {
  return {
    entry: entry("connect"),
    onclose: vi.fn(),
    onrecheck: vi.fn(),
    onremove: vi.fn(),
    onchoose: vi.fn(),
    ...overrides,
  };
}

describe("PreflightPanel", () => {
  /**
   * Scoped to the step rows: "Protocol" is also a key in the link summary below them, and a
   * bare text query cannot tell the check from the fact it established.
   */
  it("names every check and how long it took", () => {
    const { container } = render(PreflightPanel, { props: props() });
    const rows = within(container.querySelector(".rad-preflight-steps") as HTMLElement);
    for (const name of PREFLIGHT_STEPS) expect(rows.getByText(name)).toBeInTheDocument();
    expect(screen.getByText(/100 ms/)).toBeInTheDocument();
  });

  it("counts how many checks actually ran", () => {
    render(PreflightPanel, { props: props() });
    expect(screen.getByText(/4 of 4 checks ran/)).toBeInTheDocument();
  });

  /**
   * A check that never ran because an earlier one failed says so, rather than leaving a reader
   * waiting for a result that is not coming.
   */
  it("marks the checks that never ran and does not time them", () => {
    const stopped = entry("reauth", {
      steps: PREFLIGHT_STEPS.map((name, i) => ({
        name,
        state: (i === 0 ? "bad" : "skipped") as PreflightStepState,
        note: i === 0 ? "no valid sign-in for this server" : "not run",
        ms: i === 0 ? 18 : 0,
      })),
    });
    render(PreflightPanel, { props: props({ entry: stopped }) });
    expect(screen.getByText(/1 of 4 checks ran/)).toBeInTheDocument();
    expect(screen.getAllByText("not run")).toHaveLength(3);
    expect(screen.getAllByText("—").length).toBeGreaterThan(0);
  });

  it("leads with the verdict rather than the four results", () => {
    const blocked = entry("udp_blocked", {
      steps: PREFLIGHT_STEPS.map((name, i) => ({
        name,
        state: (i === 3 ? "bad" : "ok") as PreflightStepState,
        note: "",
        ms: 20,
      })),
    });
    render(PreflightPanel, { props: props({ entry: blocked }) });
    expect(screen.getByRole("status")).toHaveTextContent(/UDP 443/);
  });

  /**
   * Days until expiry, the issuing CA and the rotation window are facts about mTLS. The only
   * thing anyone can act on is signing in again, so the readout says who is signed in.
   */
  it("reports the account without exposing the certificate behind it", () => {
    render(PreflightPanel, { props: props() });
    expect(screen.getByText("Signed in as")).toBeInTheDocument();
    expect(screen.queryByText(/expir/i)).not.toBeInTheDocument();
    expect(screen.queryByText(/issuer|certificate authority/i)).not.toBeInTheDocument();
  });

  it("names the QUIC port and marks a non-standard one as a fallback", () => {
    render(PreflightPanel, { props: props({ entry: entry("connect", { quicPort: 8443 }) }) });
    expect(screen.getByText(/8443 \(fallback\)/)).toBeInTheDocument();
  });

  it("does not label 443 as a fallback", () => {
    render(PreflightPanel, { props: props() });
    expect(screen.queryByText(/fallback/)).not.toBeInTheDocument();
  });

  it("offers removal, a recheck and the primary action", async () => {
    const onremove = vi.fn();
    const onrecheck = vi.fn();
    render(PreflightPanel, { props: props({ onremove, onrecheck }) });

    await userEvent.click(screen.getByRole("button", { name: /remove/i }));
    await userEvent.click(screen.getByRole("button", { name: /^recheck/i }));
    expect(onremove).toHaveBeenCalledOnce();
    expect(onrecheck).toHaveBeenCalledWith("https://bvc.alaydriem.com");
  });

  /**
   * The plate's button is disabled for a blocked server; here it still leads somewhere — the
   * update, not the connection that would fail.
   */
  it("offers the update from here even though the plate could not", () => {
    render(PreflightPanel, {
      props: props({ entry: entry("version_mismatch", { clientTooOld: true }) }),
    });
    const go = screen.getByRole("button", { name: /update the client/i });
    expect(go).toBeEnabled();
  });

  it("offers nothing to press when the server is the one that is out of date", () => {
    render(PreflightPanel, {
      props: props({ entry: entry("version_mismatch", { clientTooOld: false }) }),
    });
    expect(screen.getByRole("button", { name: /blocked/i })).toBeDisabled();
  });

  it("does not commit to anything while checks are still running", () => {
    render(PreflightPanel, { props: props({ entry: entry("checking") }) });
    expect(screen.getByRole("button", { name: "Connect" })).toBeDisabled();
  });

  // Closed is a state of the same element rather than its absence, so the kit can animate it.
  it("stays hidden until a server is being read", () => {
    render(PreflightPanel, { props: props({ entry: null }) });
    expect(screen.getByRole("dialog", { hidden: true })).toHaveAttribute("aria-hidden", "true");
    expect(screen.queryByText("Credentials")).not.toBeInTheDocument();
  });
});
