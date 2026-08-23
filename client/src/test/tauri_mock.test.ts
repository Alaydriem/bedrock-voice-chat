import { describe, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import { info } from "@charlesportwoodii/tauri-plugin-curia";
import { invokeCalls, mockInvoke } from "./tauri";

/**
 * Proves the IPC mocks are in effect.
 *
 * These exist because the mocks previously sat inert: `vi.mock` registers nothing
 * unless something imports the module that calls it. Without these assertions a
 * future edit could unwire them again and every other test would still pass, having
 * quietly stopped testing the boundary.
 */
describe("Tauri IPC mock", () => {
  it("routes a registered command to its handler", async () => {
    mockInvoke({ probe_server: () => ({ host: "bvc.example.com" }) });
    await expect(invoke("probe_server", { server: "https://bvc.example.com" })).resolves.toEqual({
      host: "bvc.example.com",
    });
  });

  it("records what the app asked for", async () => {
    mockInvoke({ probe_server: () => ({}) });
    await invoke("probe_server", { server: "https://bvc.example.com" });
    expect(invokeCalls()).toEqual([
      { cmd: "probe_server", args: { server: "https://bvc.example.com" } },
    ]);
  });

  // A screen that starts calling something new should fail loudly here rather than
  // silently reading undefined.
  it("rejects a command nobody registered", async () => {
    mockInvoke({});
    await expect(invoke("some_new_command")).rejects.toThrow(/unmocked invoke/);
  });

  it("silences plugin-log rather than letting it reach a missing backend", () => {
    expect(() => info("nothing should throw here")).not.toThrow();
    expect(vi.isMockFunction(info)).toBe(true);
  });
});
