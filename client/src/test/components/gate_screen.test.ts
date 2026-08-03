import { describe, expect, it, vi } from "vitest";
import { render, screen, waitFor } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import GateScreen from "../../components/login/GateScreen.svelte";
import InviteMessage from "../../js/app/login/InviteMessage";

describe("GateScreen", () => {
  // The one question #222 exists to ask, and both answers must lead somewhere.
  it("asks whether a server already exists", () => {
    render(GateScreen, { props: { onhaveserver: vi.fn(), onnoserver: vi.fn() } });
    expect(screen.getByText(/is a bvc server already set up/i)).toBeInTheDocument();
  });

  it("routes someone with an address to sign in", async () => {
    const onhaveserver = vi.fn();
    render(GateScreen, { props: { onhaveserver, onnoserver: vi.fn() } });
    await userEvent.click(screen.getByText(/someone already set it up/i));
    expect(onhaveserver).toHaveBeenCalledOnce();
  });

  it("routes someone with nothing somewhere rather than nowhere", async () => {
    const onnoserver = vi.fn();
    render(GateScreen, { props: { onhaveserver: vi.fn(), onnoserver } });
    await userEvent.click(screen.getByText(/nobody has set it up yet/i));
    expect(onnoserver).toHaveBeenCalledOnce();
  });

  it("offers both answers and no dead end", () => {
    render(GateScreen, { props: { onhaveserver: vi.fn(), onnoserver: vi.fn() } });
    expect(screen.getAllByRole("button")).toHaveLength(2);
  });

  // The ring previews the answer being considered, which is the only feedback either
  // option gives. `lock` while the question is still open.
  it("holds the ring neutral until an answer is considered", () => {
    const { container } = render(GateScreen, {
      props: { onhaveserver: vi.fn(), onnoserver: vi.fn() },
    });
    expect(container.querySelector("canvas[data-rad-ring]")).toHaveAttribute(
      "data-rad-ring",
      "lock",
    );
  });

  it("brings the ring to life over the answer that has a server", async () => {
    const { container } = render(GateScreen, {
      props: { onhaveserver: vi.fn(), onnoserver: vi.fn() },
    });
    await userEvent.hover(screen.getByText(/someone already set it up/i));
    await waitFor(() =>
      expect(container.querySelector("canvas[data-rad-ring]")).toHaveAttribute(
        "data-rad-ring",
        "live",
      ),
    );
  });

  // Polled rather than asserted immediately: the ring decays into `empty` over
  // several frames instead of cutting, so the state arrives a moment after the
  // pointer does. Asserting on the next tick would pass or fail on timing.
  it("decays the ring to empty over the answer that has none", async () => {
    const { container } = render(GateScreen, {
      props: { onhaveserver: vi.fn(), onnoserver: vi.fn() },
    });
    await userEvent.hover(screen.getByText(/nobody has set it up yet/i));
    await waitFor(() =>
      expect(container.querySelector("canvas[data-rad-ring]")).toHaveAttribute(
        "data-rad-ring",
        "empty",
      ),
    );
  });

  // Hover is not available to a keyboard user, and the ring is the feedback.
  it("previews the answer on focus as well as hover", async () => {
    const { container } = render(GateScreen, {
      props: { onhaveserver: vi.fn(), onnoserver: vi.fn() },
    });
    await userEvent.tab();
    await waitFor(() =>
      expect(container.querySelector("canvas[data-rad-ring]")).toHaveAttribute(
        "data-rad-ring",
        "live",
      ),
    );
  });
});

describe("InviteMessage", () => {
  // A bare link gets ignored. This is written to be forwarded verbatim to someone
  // who has never heard of BVC.
  it("says what is being asked for, how long it takes, and what to send back", () => {
    expect(InviteMessage.TEXT).toMatch(/proximity voice chat/i);
    expect(InviteMessage.TEXT).toMatch(/fifteen minutes/i);
    expect(InviteMessage.TEXT).toMatch(/bedrockvoicechat\.com\/wiki/);
    expect(InviteMessage.TEXT).toMatch(/send me the server address/i);
  });
});
