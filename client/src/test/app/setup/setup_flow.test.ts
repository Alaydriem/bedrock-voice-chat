import { describe, expect, it } from "vitest";
import SetupFlow from "../../../js/app/setup/SetupFlow";

describe("SetupFlow.nextScreen", () => {
  it("starts at the microphone on a fresh install", () => {
    const flow = new SetupFlow();
    flow.hydrate({ microphone: false, notifications: false, devices: false });
    expect(flow.nextScreen()).toBe("microphone");
  });

  // Quitting part-way through has to return to where it stopped, not to the start.
  it("resumes at the first incomplete screen", () => {
    const flow = new SetupFlow();
    flow.hydrate({ microphone: true, notifications: false, devices: false });
    expect(flow.nextScreen()).toBe("notifications");
  });

  it("skips a completed screen in the middle", () => {
    const flow = new SetupFlow();
    flow.hydrate({ microphone: true, notifications: true, devices: false });
    expect(flow.nextScreen()).toBe("devices");
  });

  it("returns null once every screen is done", () => {
    const flow = new SetupFlow();
    flow.hydrate({ microphone: true, notifications: true, devices: true });
    expect(flow.nextScreen()).toBeNull();
    expect(flow.isComplete()).toBe(true);
  });

  it("hands back a copy, so a caller cannot mutate its state", () => {
    const flow = new SetupFlow();
    flow.hydrate({ microphone: true, notifications: false, devices: false });
    const snapshot = flow.currentState();
    snapshot.notifications = true;
    expect(flow.nextScreen()).toBe("notifications");
  });

  // An install carried forward from a build that predates the rename has no
  // setup_state. It runs setup again, which is cheap because every screen
  // re-checks the OS on arrival.
  it("treats an absent state as nothing completed", () => {
    const flow = new SetupFlow();
    expect(flow.nextScreen()).toBe("microphone");
    expect(flow.isComplete()).toBe(false);
  });
});
