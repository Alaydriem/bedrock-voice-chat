import { describe, expect, it, vi } from "vitest";
import { get } from "svelte/store";
import ScreenFlow from "../../../js/app/shell/ScreenFlow";

const SCREENS = ["intro", "gate", "login", "code", "notyet"] as const;

describe("ScreenFlow", () => {
  it("opens on the screen it was given", () => {
    const flow = new ScreenFlow({ screens: SCREENS, initial: "login", steps: 4 });
    expect(get(flow.screen)).toBe("login");
  });

  it("moves between known screens", () => {
    const flow = new ScreenFlow({ screens: SCREENS, initial: "gate", steps: 4 });
    flow.go("notyet");
    expect(get(flow.screen)).toBe("notyet");
  });

  // A typo'd screen name must not blank the frame.
  it("ignores an unknown screen", () => {
    const flow = new ScreenFlow({ screens: SCREENS, initial: "gate", steps: 4 });
    flow.go("dashboard");
    expect(get(flow.screen)).toBe("gate");
  });

  it("reports each screen change once", () => {
    const onScreen = vi.fn();
    const flow = new ScreenFlow({ screens: SCREENS, initial: "intro", steps: 4, onScreen });
    flow.go("gate");
    flow.go("gate");
    expect(onScreen.mock.calls).toEqual([["gate"]]);
  });

  it("clamps steps rather than running past either end", () => {
    const flow = new ScreenFlow({ screens: SCREENS, initial: "intro", steps: 4 });
    flow.backStep();
    expect(get(flow.step)).toBe(1);
    flow.goStep(99);
    expect(get(flow.step)).toBe(4);
  });

  it("knows when it is on the last step, which is what swaps the button label", () => {
    const flow = new ScreenFlow({ screens: SCREENS, initial: "intro", steps: 4 });
    expect(flow.isLastStep()).toBe(false);
    flow.goStep(4);
    expect(flow.isLastStep()).toBe(true);
  });

  // Re-entering the introduction from the revisit link has to start at one: arriving
  // back at step three of an explanation you chose to re-read is not a place anyone
  // asked to be.
  it("resets to step one when a screen is re-entered", () => {
    const flow = new ScreenFlow({ screens: SCREENS, initial: "intro", steps: 4 });
    flow.goStep(3);
    flow.go("gate");
    flow.go("intro");
    expect(get(flow.step)).toBe(1);
  });

  it("tolerates a null node when asked to replay the stagger", () => {
    const flow = new ScreenFlow({ screens: SCREENS, initial: "intro", steps: 4 });
    expect(() => flow.restage(null)).not.toThrow();
  });

  // A flow with no stepped screen still has to answer the step questions.
  it("defaults to a single step when none are declared", () => {
    const flow = new ScreenFlow({ screens: ["microphone", "devices"], initial: "microphone" });
    expect(flow.total).toBe(1);
    expect(flow.isLastStep()).toBe(true);
  });
});
