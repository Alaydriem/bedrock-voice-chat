import { describe, expect, it, vi } from "vitest";
import { render, screen } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import SignInScreen from "../../components/login/SignInScreen.svelte";
import type { ResolveVerdict } from "../../js/app/login/AddressResolver";

const editing: ResolveVerdict = {
  state: "editing",
  ring: "empty",
  line: "○ Resolving",
  caption: "RESOLVING",
  busy: false,
};
const measuring: ResolveVerdict = {
  state: "editing",
  ring: "empty",
  line: "Resolving",
  caption: "RESOLVING",
  busy: true,
};
const ok: ResolveVerdict = {
  state: "ok",
  ring: "lock",
  line: "● Resolved · 41 ms",
  caption: "RESOLVED · 41 MS",
  busy: false,
};
const bad: ResolveVerdict = {
  state: "bad",
  ring: "empty",
  line: "✕ Nothing at that address",
  caption: "NO RESPONSE",
  busy: false,
};

function props(overrides: Record<string, unknown> = {}) {
  return {
    address: "bvc.example.com",
    verdict: editing,
    appVersion: "1.0.0-beta.20",
    oninput: vi.fn(),
    onconnect: vi.fn(),
    onprivacy: vi.fn(),
    onrevisit: vi.fn(),
    ...overrides,
  };
}

describe("SignInScreen", () => {
  it("offers a labelled address field carrying the current value", () => {
    render(SignInScreen, { props: props() });
    expect(screen.getByLabelText("Server address")).toHaveValue("bvc.example.com");
  });

  it("reports each keystroke so the probe can debounce", async () => {
    const oninput = vi.fn();
    render(SignInScreen, { props: props({ address: "", oninput }) });
    await userEvent.type(screen.getByLabelText("Server address"), "b");
    expect(oninput).toHaveBeenCalled();
  });

  it("shows the measured round trip once the address resolves", () => {
    render(SignInScreen, { props: props({ verdict: ok }) });
    expect(screen.getByText("● Resolved · 41 ms")).toBeInTheDocument();
  });

  it("says specifically what failed rather than that something did", () => {
    render(SignInScreen, { props: props({ verdict: bad }) });
    expect(screen.getByText("✕ Nothing at that address")).toBeInTheDocument();
  });

  /**
   * The spinner goes on this line and not on the ring. On a phone the ring is above the fold,
   * so a measurement that showed itself only there would not show itself at all.
   */
  it("spins beside the resolve line while a measurement is in flight", () => {
    const { container } = render(SignInScreen, { props: props({ verdict: measuring }) });
    expect(container.querySelector(".rad-resolve .rad-icon-spin")).not.toBeNull();
  });

  it("shows nothing spinning once a verdict has landed", () => {
    for (const verdict of [editing, ok, bad]) {
      const { container, unmount } = render(SignInScreen, { props: props({ verdict }) });
      expect(container.querySelector(".rad-icon-spin")).toBeNull();
      unmount();
    }
  });

  // The rule the whole probe design rests on. A slow or blocked probe must never be
  // the reason someone cannot sign in.
  it("keeps sign-in available in every verdict state", () => {
    for (const verdict of [editing, ok, bad]) {
      const { unmount } = render(SignInScreen, { props: props({ verdict }) });
      expect(screen.getByRole("button", { name: /sign in with microsoft/i })).toBeEnabled();
      unmount();
    }
  });

  it("hands off to the browser when sign-in is pressed", async () => {
    const onconnect = vi.fn();
    render(SignInScreen, { props: props({ onconnect }) });
    await userEvent.click(screen.getByRole("button", { name: /sign in with microsoft/i }));
    expect(onconnect).toHaveBeenCalledOnce();
  });

  // #222 asks for a way back into the explainer after first launch, so quitting
  // part-way through is not the only chance to read it.
  it("offers a way back into the introduction", async () => {
    const onrevisit = vi.fn();
    render(SignInScreen, { props: props({ onrevisit }) });
    await userEvent.click(screen.getByRole("button", { name: /what is this/i }));
    expect(onrevisit).toHaveBeenCalledOnce();
  });

  // Reached from "Add a server" on a device that is already signed in somewhere. Without a
  // way back this screen is a one-way door: signing in again is the only exit.
  it("offers the way back it was given", async () => {
    const onback = vi.fn();
    render(SignInScreen, { props: props({ onback, backLabel: "Cancel" }) });
    await userEvent.click(screen.getByRole("button", { name: /^cancel$/i }));
    expect(onback).toHaveBeenCalledOnce();
  });

  // A first launch has nowhere to go back to, and an exit that leads back here is worse
  // than none.
  it("shows no way back when it was given none", () => {
    render(SignInScreen, { props: props() });
    expect(screen.queryByRole("button", { name: /^cancel$/i })).not.toBeInTheDocument();
  });

  it("reaches the privacy notice", async () => {
    const onprivacy = vi.fn();
    render(SignInScreen, { props: props({ onprivacy }) });
    await userEvent.click(screen.getByRole("button", { name: /privacy notice/i }));
    expect(onprivacy).toHaveBeenCalledOnce();
  });

  it("shows the running version, because a bug report needs it", () => {
    render(SignInScreen, { props: props() });
    expect(screen.getByText("v1.0.0-beta.20")).toBeInTheDocument();
  });

  // The sign-in code flow is deliberately unadvertised: a visible option sends
  // people down the slower path when the fast one would have worked.
  it("advertises no route to the sign-in code flow", () => {
    render(SignInScreen, { props: props() });
    expect(screen.queryByText(/sign-in code/i)).not.toBeInTheDocument();
    expect(screen.queryByText(/login with code/i)).not.toBeInTheDocument();
  });

  // Coloured means confirmed reachable. Anything else is grey.
  it("lights the ring only on a confirmed measurement", () => {
    const { container, unmount } = render(SignInScreen, { props: props({ verdict: ok }) });
    expect(container.querySelector("canvas[data-rad-ring]")).toHaveAttribute(
      "data-rad-ring",
      "lock",
    );
    unmount();

    const second = render(SignInScreen, { props: props({ verdict: bad }) });
    expect(second.container.querySelector("canvas[data-rad-ring]")).toHaveAttribute(
      "data-rad-ring",
      "empty",
    );
  });
});
