import { describe, expect, it, vi } from "vitest";
import { render, screen, waitFor } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import MicrophoneScreen from "../../components/setup/MicrophoneScreen.svelte";
import type { PermissionFlowState } from "../../js/app/PermissionRequestManager";

function props(overrides: { state?: PermissionFlowState; step?: number; total?: number; onrequest?: () => void } = {}) {
  return {
    state: "idle" as PermissionFlowState,
    step: 1,
    total: 3,
    onrequest: vi.fn(),
    ...overrides,
  };
}

describe("MicrophoneScreen", () => {
  it("says what it wants and why before asking for it", () => {
    render(MicrophoneScreen, { props: props() });
    expect(screen.getByText(/your microphone\./i)).toBeInTheDocument();
    expect(screen.getByText(/nothing is recorded/i)).toBeInTheDocument();
  });

  it("asks only when the user chooses to", async () => {
    const onrequest = vi.fn();
    render(MicrophoneScreen, { props: props({ onrequest }) });
    await userEvent.click(screen.getByRole("button", { name: /allow microphone access/i }));
    expect(onrequest).toHaveBeenCalledOnce();
  });

  // The OS prompt is modal and the app looks frozen behind it. This is exactly the
  // wait the loader's status line exists for, and it must not be four seconds late.
  it("explains itself while waiting on the OS prompt", async () => {
    render(MicrophoneScreen, { props: props({ state: "requesting" }) });
    await waitFor(() => expect(screen.getByRole("status")).toBeInTheDocument());
  });

  // A denial is recoverable through system settings, and saying so is the difference
  // between a dead end and a detour.
  it("tells the user what to do when permission was refused", () => {
    render(MicrophoneScreen, { props: props({ state: "denied" }) });
    expect(screen.getByText(/system settings/i)).toBeInTheDocument();
  });

  // Voice chat without a microphone is not a product, so there is no way past this
  // one. Notifications are a different matter.
  it("offers no way to skip", () => {
    render(MicrophoneScreen, { props: props({ state: "denied" }) });
    expect(screen.queryByRole("button", { name: /skip/i })).not.toBeInTheDocument();
  });

  it("shows where the reader is in setup", () => {
    render(MicrophoneScreen, { props: props({ step: 1, total: 3 }) });
    expect(screen.getByText("01 / 03")).toBeInTheDocument();
  });
});
