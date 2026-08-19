import { beforeEach, describe, expect, it, vi } from "vitest";
import { BOOT_STEPS, BootProgress } from "../../../js/app/shell/BootProgress";

vi.mock("@tauri-apps/plugin-log", () => ({
  debug: vi.fn(),
  info: vi.fn(),
}));

type Written = { name: string; state: string; note?: string };

function receiver(): Written[] {
  const written: Written[] = [];
  window.__bvcBootStep = (name, state, note) => written.push({ name, state, note });
  return written;
}

/**
 * The overlay runs outside the app bundle, so the only thing binding these two halves
 * together is the shape of this call. Nothing else fails when it drifts — the lights
 * simply stop moving, on the screen that exists for when something has gone wrong.
 */
describe("BootProgress", () => {
  beforeEach(() => {
    // The reporter is shared for the launch; each test gets a launch of its own.
    (BootProgress as unknown as { instance: BootProgress | null }).instance = null;
    delete window.__bvcBootStep;
  });

  it("reports a phase and its note to the overlay", () => {
    const written = receiver();

    BootProgress.shared().step("Voice path", "warn", "udp blocked");

    expect(written).toEqual([{ name: "Voice path", state: "warn", note: "udp blocked" }]);
  });

  // The dashboard reports from several places, and a phase that has not moved must not
  // cost a DOM write on a screen that is already painting a ring every frame.
  it("writes a repeated state only once", () => {
    const written = receiver();
    const progress = BootProgress.shared();

    progress.step("Server", "running");
    progress.step("Server", "running");

    expect(written).toHaveLength(1);
  });

  it("writes again when the note changes", () => {
    const written = receiver();
    const progress = BootProgress.shared();

    progress.step("Server", "ok");
    progress.step("Server", "ok", "cached");

    expect(written.map((entry) => entry.note)).toEqual([undefined, "cached"]);
  });

  // A reader left on a pending light is waiting for a result that is not coming, which
  // is the whole reason `skipped` is a state distinct from `pending`.
  it("marks every phase from the named one onward as skipped", () => {
    const written = receiver();

    BootProgress.shared().skipFrom("Permissions");

    expect(written.map((entry) => entry.name)).toEqual(["Permissions", "Voice path", "Audio"]);
    expect(written.every((entry) => entry.state === "skipped")).toBe(true);
  });

  it("skips nothing for a phase it does not know", () => {
    const written = receiver();

    BootProgress.shared().skipFrom("Nowhere" as (typeof BOOT_STEPS)[number]);

    expect(written).toEqual([]);
  });

  // Every route without the overlay, and every call after it is dismissed. Progress
  // reporting must never be the thing that breaks a launch.
  it("is inert when the overlay is not there", () => {
    expect(() => BootProgress.shared().step("Session", "ok")).not.toThrow();
  });
});
