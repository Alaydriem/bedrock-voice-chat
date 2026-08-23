import { describe, expect, it, vi } from "vitest";
import { render, screen } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import CodeScreen from "../../components/login/CodeScreen.svelte";

function props(overrides: Record<string, unknown> = {}) {
  return {
    server: "https://bvc.example.com",
    error: "",
    isSubmitting: false,
    appVersion: "1.0.0-beta.20",
    onsubmit: vi.fn(),
    onback: vi.fn(),
    ...overrides,
  };
}

describe("CodeScreen", () => {
  it("says which server the code is for", () => {
    render(CodeScreen, { props: props() });
    expect(screen.getByText("https://bvc.example.com")).toBeInTheDocument();
  });

  it("submits what the user typed", async () => {
    const onsubmit = vi.fn();
    render(CodeScreen, { props: props({ onsubmit }) });
    await userEvent.type(screen.getByLabelText(/^code$/i), "xkcd4417");
    await userEvent.click(screen.getByRole("button", { name: /^sign in$/i }));
    expect(onsubmit).toHaveBeenCalledWith({ code: "xkcd4417" });
  });

  /**
   * The server resolves the player and the game from the code, so anything else on this
   * screen asks for something it already knows. The payload assertion above catches a
   * field being sent; this one catches the screen growing an input.
   */
  it("asks for nothing but the code", () => {
    render(CodeScreen, { props: props() });
    expect(screen.queryByLabelText(/gamertag/i)).not.toBeInTheDocument();
    expect(screen.queryByRole("group", { name: /game/i })).not.toBeInTheDocument();
    expect(screen.getAllByRole("textbox")).toHaveLength(1);
  });

  it("surfaces a failure instead of leaving the user guessing", () => {
    render(CodeScreen, {
      props: props({ error: "That code was not accepted. Check it and try again." }),
    });
    expect(
      screen.getByText("That code was not accepted. Check it and try again."),
    ).toBeInTheDocument();
  });

  // A second submission while the first is in flight would post the same code twice.
  it("blocks a second submission while one is in flight", () => {
    render(CodeScreen, { props: props({ isSubmitting: true }) });
    expect(screen.getByRole("button", { name: /^sign in$/i })).toBeDisabled();
  });

  // Two of them, deliberately: the form's own sits below the field and its buttons and
  // can be scrolled off, so the chrome carries one that is always reachable.
  it("offers a way back to normal sign-in from anywhere on the screen", async () => {
    const onback = vi.fn();
    render(CodeScreen, { props: props({ onback }) });
    const backs = screen.getAllByRole("button", { name: /back to sign in/i });
    expect(backs.length).toBeGreaterThanOrEqual(2);
    for (const back of backs) {
      await userEvent.click(back);
    }
    expect(onback).toHaveBeenCalledTimes(backs.length);
  });
});
