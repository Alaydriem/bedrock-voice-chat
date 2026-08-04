import { describe, expect, it, vi } from "vitest";
import { render, screen } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import ServerListScreen from "../../components/server/ServerListScreen.svelte";
import { PREFLIGHT_STEPS } from "../../js/app/server/preflight/PreflightStepName";
import type { PreflightStepState } from "../../js/app/server/preflight/PreflightStepState";
import type { RosterStatus } from "../../js/app/server/RosterStatus";
import type { ServerRosterEntry } from "../../js/app/server/ServerRosterEntry";

function entry(host: string, status: RosterStatus, overrides: Partial<ServerRosterEntry> = {}) {
  const settled: PreflightStepState = status === "checking" ? "pending" : "ok";
  return {
    server: `https://${host}`,
    host,
    player: "Alaydriem",
    game: "minecraft",
    status,
    steps: PREFLIGHT_STEPS.map((name) => ({ name, state: settled, note: "", ms: 12 })),
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
    entries: [entry("bvc.alaydriem.com", "connect")],
    isRefreshing: false,
    onchoose: vi.fn(),
    onopen: vi.fn(),
    onadd: vi.fn(),
    onrecheckall: vi.fn(),
    ...overrides,
  };
}

describe("ServerListScreen", () => {
  it("identifies each server by its address", () => {
    render(ServerListScreen, { props: props() });
    expect(screen.getByText("bvc.alaydriem.com")).toBeInTheDocument();
  });

  // Which account is saved for a server matters when the same person has more than one.
  it("says which account is signed in to each server", () => {
    render(ServerListScreen, { props: props() });
    expect(screen.getByText(/signed in as Alaydriem/)).toBeInTheDocument();
  });

  /**
   * The rule the whole screen is built on: identification wants recognition rather than
   * recall, so there is no ring here. It is the empty state and the status oscilloscope,
   * never a roster.
   */
  it("draws no ring", () => {
    const { container } = render(ServerListScreen, { props: props() });
    expect(container.querySelector(".rad-ring")).toBeNull();
    expect(container.querySelector(".rad-visual")).toBeNull();
  });

  it("hands the chosen server back by its stored url", async () => {
    const onchoose = vi.fn();
    render(ServerListScreen, { props: props({ onchoose }) });
    await userEvent.click(screen.getByRole("button", { name: "Connect" }));
    expect(onchoose).toHaveBeenCalledWith("https://bvc.alaydriem.com");
  });

  // The strip is the way into the readout, and the readout is where removal lives.
  it("opens the preflight readout from the strip", async () => {
    const onopen = vi.fn();
    const { container } = render(ServerListScreen, { props: props({ onopen }) });
    await userEvent.click(container.querySelector(".rad-preflight") as HTMLElement);
    expect(onopen).toHaveBeenCalledWith("https://bvc.alaydriem.com");
  });

  it("shows one block per check on every plate", () => {
    const { container } = render(ServerListScreen, { props: props() });
    expect(container.querySelectorAll(".rad-preflight__blocks i")).toHaveLength(
      PREFLIGHT_STEPS.length,
    );
  });

  it("totals the time a finished preflight took", () => {
    render(ServerListScreen, { props: props() });
    expect(screen.getByText("48 ms")).toBeInTheDocument();
  });

  it("says it is checking rather than showing a total that is not final", () => {
    render(ServerListScreen, {
      props: props({ entries: [entry("a.example.com", "checking")] }),
    });
    expect(screen.getByText("checking")).toBeInTheDocument();
  });

  /**
   * Adding a server is one of the things you can pick on a screen whose whole job is picking,
   * so it is a tile in the grid as well as a button in the chrome.
   */
  it("offers adding a server as a tile in the grid", async () => {
    const onadd = vi.fn();
    render(ServerListScreen, { props: props({ onadd }) });
    const buttons = screen.getAllByRole("button", { name: /add a server/i });
    expect(buttons.length).toBe(2);
    await userEvent.click(buttons[1]);
    expect(onadd).toHaveBeenCalledOnce();
  });

  it("shows every saved server, not just the working ones", () => {
    const { container } = render(ServerListScreen, {
      props: props({
        entries: [
          entry("a.example.com", "connect"),
          entry("b.example.com", "udp_blocked"),
          entry("c.example.com", "reauth"),
        ],
      }),
    });
    expect(container.querySelectorAll(".rad-server")).toHaveLength(3);
  });

  // Voice is the product: a server with no UDP path has nothing worth connecting to.
  it("never offers a connect on a voice-blocked server", () => {
    render(ServerListScreen, {
      props: props({ entries: [entry("b.example.com", "udp_blocked")] }),
    });
    expect(screen.queryByRole("button", { name: "Connect" })).not.toBeInTheDocument();
    expect(screen.getByText("Voice blocked")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Recheck" })).toBeInTheDocument();
  });

  it("counts the servers above the list", () => {
    render(ServerListScreen, {
      props: props({
        entries: [entry("a.example.com", "connect"), entry("b.example.com", "connect")],
      }),
    });
    expect(screen.getByText(/2 servers/)).toBeInTheDocument();
  });

  it("counts one server without pluralising it", () => {
    render(ServerListScreen, { props: props() });
    expect(screen.getByText(/1 server/)).toBeInTheDocument();
  });

  /**
   * The glance that says whether the screen is worth reading closely. The plates already name
   * the servers; this counts the states.
   */
  it("tallies the states in the bar under the list", () => {
    render(ServerListScreen, {
      props: props({
        entries: [
          entry("a.example.com", "connect"),
          entry("b.example.com", "connect"),
          entry("c.example.com", "udp_blocked"),
        ],
      }),
    });
    expect(screen.getByText(/2\s+ready/)).toBeInTheDocument();
    expect(screen.getByText(/1\s+voice blocked/)).toBeInTheDocument();
  });

  it("rechecks everything from either control", async () => {
    const onrecheckall = vi.fn();
    render(ServerListScreen, { props: props({ onrecheckall }) });
    await userEvent.click(screen.getByRole("button", { name: /recheck every server/i }));
    await userEvent.click(screen.getByRole("button", { name: /recheck all/i }));
    expect(onrecheckall).toHaveBeenCalledTimes(2);
  });

  it("stops accepting rechecks while one is running", () => {
    render(ServerListScreen, { props: props({ isRefreshing: true }) });
    expect(screen.getByRole("button", { name: /rechecking/i })).toBeDisabled();
  });
});
