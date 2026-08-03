import { describe, expect, it } from "vitest";
import LaunchGate from "../../../js/app/login/LaunchGate";

const none = new URLSearchParams("");

describe("LaunchGate.resolveEntry", () => {
  // Someone with no server has not been onboarded. Defaulting the other way sends a
  // brand-new user to a credential prompt for a server they do not have.
  it("onboards an install with no servers", () => {
    expect(LaunchGate.resolveEntry(false, none)).toBe("intro");
  });

  it("goes straight to sign in when servers exist", () => {
    expect(LaunchGate.resolveEntry(true, none)).toBe("login");
  });

  it("skips the introduction when adding a server", () => {
    expect(LaunchGate.resolveEntry(true, new URLSearchParams("addserver"))).toBe("login");
  });

  it("skips the introduction when re-authenticating a known server", () => {
    expect(
      LaunchGate.resolveEntry(true, new URLSearchParams("server=bvc.example.com&reauth=true")),
    ).toBe("login");
  });

  // Belt and braces: an install arriving with these params necessarily has servers,
  // but a future caller must not be able to drop someone into an explainer mid-task.
  it("never onboards a launch that arrived with a server in hand", () => {
    expect(LaunchGate.resolveEntry(false, new URLSearchParams("server=bvc.example.com"))).toBe(
      "login",
    );
  });

  // A returning user whose credentials vanished still has a server list, so they
  // land on sign-in rather than sitting through an explanation they have read.
  it("does not re-onboard a user whose credentials went missing", () => {
    expect(LaunchGate.resolveEntry(true, new URLSearchParams("logout=true"))).toBe("login");
  });
});
