import { describe, expect, it, vi } from "vitest";
import { render, screen, within } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import ServerListScreen from "../../components/server/ServerListScreen.svelte";
import type { RosterStatus } from "../../js/app/server/RosterStatus";
import type { ServerRosterEntry } from "../../js/app/server/ServerRosterEntry";

function entry(host: string, status: RosterStatus, overrides: Partial<ServerRosterEntry> = {}) {
  return {
    server: `https://${host}`,
    host,
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

function props(overrides: Record<string, unknown> = {}) {
  return {
    entries: [entry("bvc.alaydriem.com", "connect")],
    isRefreshing: false,
    appVersion: "1.0.0",
    onchoose: vi.fn(),
    onforget: vi.fn(),
    onadd: vi.fn(),
    onrefresh: vi.fn(),
    onsettings: vi.fn(),
    ...overrides,
  };
}

/**
 * The rows alone. The visual pane names the server it is reading too, so a bare text query
 * cannot tell "the list shows this server" from "the ring is pointed at it".
 */
function list(container: HTMLElement) {
  return within(container.querySelector(".srv-list") as HTMLElement);
}

describe("ServerListScreen", () => {
  it("identifies each server by its address", () => {
    const { container } = render(ServerListScreen, { props: props() });
    expect(list(container).getByText("bvc.alaydriem.com")).toBeInTheDocument();
  });

  // Which account is saved for a server matters when the same person has more than one.
  it("says which player is signed in to each server", () => {
    render(ServerListScreen, { props: props() });
    expect(screen.getByText(/Alaydriem/)).toBeInTheDocument();
  });

  it("hands the chosen server back by its stored url", async () => {
    const onchoose = vi.fn();
    render(ServerListScreen, { props: props({ onchoose }) });
    await userEvent.click(screen.getByRole("button", { name: "Join" }));
    expect(onchoose).toHaveBeenCalledWith("https://bvc.alaydriem.com");
  });

  /**
   * Removing is destructive and irreversible from here, so the row raises it rather than
   * doing it. The confirm lives on the page.
   */
  it("asks rather than acts when a server is forgotten", async () => {
    const onforget = vi.fn();
    render(ServerListScreen, { props: props({ onforget }) });
    await userEvent.click(screen.getByRole("button", { name: /forget bvc\.alaydriem\.com/i }));
    expect(onforget).toHaveBeenCalledOnce();
  });

  it("offers a way to add another server", async () => {
    const onadd = vi.fn();
    render(ServerListScreen, { props: props({ onadd }) });
    await userEvent.click(screen.getByRole("button", { name: /add a server/i }));
    expect(onadd).toHaveBeenCalledOnce();
  });

  it("shows every saved server, not just the working ones", () => {
    const { container } = render(ServerListScreen, {
      props: props({
        entries: [
          entry("a.example.com", "connect"),
          entry("b.example.com", "unreachable"),
          entry("c.example.com", "reauth"),
        ],
      }),
    });
    const rows = list(container);
    expect(rows.getByText("a.example.com")).toBeInTheDocument();
    expect(rows.getByText("b.example.com")).toBeInTheDocument();
    expect(rows.getByText("c.example.com")).toBeInTheDocument();
  });

  /**
   * The pane rests on the server that can be joined rather than on whichever one is stored
   * first, so the visual opens on the answer instead of on a dead host.
   */
  it("reads the joinable server in the visual, not the first row", () => {
    render(ServerListScreen, {
      props: props({
        entries: [entry("dead.example.com", "unreachable"), entry("live.example.com", "connect")],
      }),
    });
    expect(screen.getByText("READY TO JOIN")).toBeInTheDocument();
  });

  // A server that is down and a sign-in that has lapsed are different problems with
  // different fixes, and the list is where that distinction has to survive.
  it("does not offer a sign-in for a server that is not answering", () => {
    render(ServerListScreen, {
      props: props({ entries: [entry("b.example.com", "unreachable")] }),
    });
    expect(screen.queryByRole("button", { name: "Sign in" })).not.toBeInTheDocument();
    expect(screen.getByText(/not answering/i)).toBeInTheDocument();
  });

  it("explains a server on an older protocol instead of offering an action", () => {
    render(ServerListScreen, {
      props: props({
        entries: [
          entry("b.example.com", "version_mismatch", {
            serverVersion: "2.0.0",
            clientVersion: "2.1.0",
          }),
        ],
      }),
    });
    expect(screen.getByText(/has to update it/i)).toBeInTheDocument();
  });

  it("says it is working while every server is re-checked", () => {
    render(ServerListScreen, { props: props({ isRefreshing: true }) });
    expect(screen.getByText(/checking every server/i)).toBeInTheDocument();
  });

  it("counts the saved servers in the chrome", () => {
    render(ServerListScreen, {
      props: props({
        entries: [entry("a.example.com", "connect"), entry("b.example.com", "connect")],
      }),
    });
    expect(screen.getByText(/2 servers saved/i)).toBeInTheDocument();
  });

  it("counts one server without pluralising it", () => {
    render(ServerListScreen, { props: props() });
    expect(screen.getByText(/1 server saved/i)).toBeInTheDocument();
  });
});
