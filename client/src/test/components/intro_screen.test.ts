import { describe, expect, it, vi } from "vitest";
import { render, screen } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import IntroScreen from "../../components/login/IntroScreen.svelte";

function props(overrides: Record<string, unknown> = {}) {
  return {
    step: 1,
    onstep: vi.fn(),
    onnext: vi.fn(),
    onback: vi.fn(),
    onskip: vi.fn(),
    ...overrides,
  };
}

describe("IntroScreen", () => {
  it("opens on proximity, which is the thing that sells the product", () => {
    render(IntroScreen, { props: props() });
    expect(screen.getByText(/01 · Proximity/)).toBeInTheDocument();
  });

  it("shows only the current step's copy", () => {
    render(IntroScreen, { props: props({ step: 2 }) });
    expect(screen.getByText(/02 · Channels/)).toBeInTheDocument();
    expect(screen.queryByText(/01 · Proximity/)).not.toBeInTheDocument();
  });

  it("carries the design's copy verbatim on the first step", () => {
    render(IntroScreen, { props: props() });
    expect(screen.getByText(/Walk up to someone/)).toBeInTheDocument();
  });

  // Back on step one would go nowhere, and an inert control is worse than none.
  it("hides Back on the first step", () => {
    render(IntroScreen, { props: props({ step: 1 }) });
    expect(screen.queryByRole("button", { name: /^back$/i })).not.toBeInTheDocument();
  });

  it("offers Back once there is somewhere to go", async () => {
    const onback = vi.fn();
    render(IntroScreen, { props: props({ step: 2, onback }) });
    await userEvent.click(screen.getByRole("button", { name: /^back$/i }));
    expect(onback).toHaveBeenCalledOnce();
  });

  // The last step's forward button leaves the introduction, so it must not promise
  // another step.
  it("says Next mid-run and Continue at the end", () => {
    const { unmount } = render(IntroScreen, { props: props({ step: 3 }) });
    expect(screen.getByRole("button", { name: /^next$/i })).toBeInTheDocument();
    unmount();
    render(IntroScreen, { props: props({ step: 4 }) });
    expect(screen.getByRole("button", { name: /^continue$/i })).toBeInTheDocument();
  });

  // Four steps is the ceiling; every added step multiplies the skip rate, and
  // skipping must always be available.
  it("always lets the reader skip out", async () => {
    const onskip = vi.fn();
    render(IntroScreen, { props: props({ step: 2, onskip }) });
    await userEvent.click(screen.getByRole("button", { name: /^skip$/i }));
    expect(onskip).toHaveBeenCalledOnce();
  });

  it("exposes exactly four steps, with the current one marked", () => {
    render(IntroScreen, { props: props({ step: 3 }) });
    expect(screen.getByText("03 / 04")).toBeInTheDocument();
    expect(screen.getByLabelText("Step 3")).toHaveAttribute("aria-current", "step");
  });

  it("moves forward when the forward button is pressed", async () => {
    const onnext = vi.fn();
    render(IntroScreen, { props: props({ step: 2, onnext }) });
    await userEvent.click(screen.getByRole("button", { name: /^next$/i }));
    expect(onnext).toHaveBeenCalledOnce();
  });

  it("lets a dot jump straight to a step", async () => {
    const onstep = vi.fn();
    render(IntroScreen, { props: props({ step: 3, onstep }) });
    await userEvent.click(screen.getByLabelText("Step 1"));
    expect(onstep).toHaveBeenCalledWith(1);
  });
});
