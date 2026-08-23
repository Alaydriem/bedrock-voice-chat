import { describe, expect, it, vi } from "vitest";
import { render, screen } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import NotificationsScreen from "../../components/setup/NotificationsScreen.svelte";
import type { PermissionFlowState } from "../../js/app/PermissionRequestManager";

function props(
  overrides: {
    state?: PermissionFlowState;
    step?: number;
    total?: number;
    onrequest?: () => void;
  } = {},
) {
  return {
    state: "idle" as PermissionFlowState,
    step: 2,
    total: 3,
    onrequest: vi.fn(),
    ...overrides,
  };
}

describe("NotificationsScreen", () => {
  /**
   * The reason given has to be background audio, not alerts. Both platforms require a
   * notification for an app to hold the microphone off screen, and playing Minecraft
   * means BVC is always off screen — so someone told these were about knowing when a
   * friend joined would reasonably refuse and then find voice cuts out in game.
   */
  it("gives background audio as the reason", () => {
    render(NotificationsScreen, { props: props() });
    expect(screen.getByText(/hold the microphone in the\s+background/i)).toBeInTheDocument();
    expect(screen.getByText(/background audio depends on it/i)).toBeInTheDocument();
  });

  it("asks only when the user chooses to", async () => {
    const onrequest = vi.fn();
    render(NotificationsScreen, { props: props({ onrequest }) });
    await userEvent.click(screen.getByRole("button", { name: /allow notifications/i }));
    expect(onrequest).toHaveBeenCalledOnce();
  });

  /**
   * This step is not skippable, so the screen must offer no way past it. A "Not now"
   * here would advance setup into a state where voice dies as soon as the user switches
   * to the game, which is indistinguishable from the app being broken.
   */
  it("offers no way to decline", () => {
    render(NotificationsScreen, { props: props() });
    expect(screen.queryByRole("button", { name: /not now/i })).not.toBeInTheDocument();
    expect(screen.queryByRole("button", { name: /skip/i })).not.toBeInTheDocument();
    expect(screen.queryByText(/optional/i)).not.toBeInTheDocument();
  });

  it("says it is required rather than leaving it ambiguous", () => {
    render(NotificationsScreen, { props: props() });
    expect(screen.getByText(/required/i)).toBeInTheDocument();
  });

  // A refusal is recoverable, not terminal: the retry is what a user who granted it in
  // system settings comes back to.
  it("offers a retry after a refusal", async () => {
    const onrequest = vi.fn();
    render(NotificationsScreen, { props: props({ state: "denied", onrequest }) });
    expect(screen.getByText(/notification access was refused/i)).toBeInTheDocument();
    await userEvent.click(screen.getByRole("button", { name: /allow notifications/i }));
    expect(onrequest).toHaveBeenCalledOnce();
  });

  it("shows where the reader is in setup", () => {
    render(NotificationsScreen, { props: props() });
    expect(screen.getByText("02 / 03")).toBeInTheDocument();
  });
});
