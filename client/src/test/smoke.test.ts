import { describe, expect, it, vi } from "vitest";
import { render, screen } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import Button from "$radial/components/Button.svelte";
import Ring from "$radial/components/Ring.svelte";

describe("component harness", () => {
  it("renders a kit component and finds it by accessible role", () => {
    render(Button);
    expect(screen.getByRole("button")).toBeInTheDocument();
  });

  it("delivers a real click to a Svelte 5 handler", async () => {
    const onclick = vi.fn();
    render(Button, { props: { onclick } });
    await userEvent.click(screen.getByRole("button"));
    expect(onclick).toHaveBeenCalledOnce();
  });

  it("reflects props into the rendered DOM", () => {
    render(Button, { props: { disabled: true, variant: "primary" } });
    const button = screen.getByRole("button");
    expect(button).toBeDisabled();
    expect(button.className).toContain("rad-btn--primary");
  });

  // If this throws, the getContext stub is wrong and every screen test that
  // renders a frame will fail for a reason that has nothing to do with the screen.
  it("survives a canvas-bearing component, which happy-dom cannot paint", () => {
    expect(() => render(Ring, { props: { mode: "empty" } })).not.toThrow();
  });

  it("hands the visual its mode, which is what tests assert instead of pixels", () => {
    const { container } = render(Ring, { props: { mode: "lock" } });
    expect(container.querySelector("canvas")).toHaveAttribute("data-rad-ring", "lock");
  });
});
