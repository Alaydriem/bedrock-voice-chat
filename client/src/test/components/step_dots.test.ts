import { describe, expect, it, vi } from "vitest";
import { render, screen } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import StepDots from "../../components/shell/StepDots.svelte";

describe("StepDots", () => {
  it("offers one target per step", () => {
    render(StepDots, { props: { step: 1, total: 4 } });
    expect(screen.getAllByRole("button")).toHaveLength(4);
  });

  it("tells assistive technology which step is current", () => {
    render(StepDots, { props: { step: 3, total: 4 } });
    expect(screen.getByLabelText("Step 3")).toHaveAttribute("aria-current", "step");
    expect(screen.getByLabelText("Step 1")).not.toHaveAttribute("aria-current");
  });

  it("reads the position out in the design's zero-padded form", () => {
    render(StepDots, { props: { step: 2, total: 4 } });
    expect(screen.getByText("02 / 04")).toBeInTheDocument();
  });

  it("lets the reader jump back to a step they have already seen", async () => {
    const onselect = vi.fn();
    render(StepDots, { props: { step: 3, total: 4, onselect } });
    await userEvent.click(screen.getByLabelText("Step 1"));
    expect(onselect).toHaveBeenCalledWith(1);
  });
});
