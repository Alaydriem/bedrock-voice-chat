import "@testing-library/jest-dom/vitest";
import { cleanup } from "@testing-library/svelte";
import { afterEach, vi } from "vitest";

// Registers the Tauri IPC mocks for every test file. Imported here rather than per
// test because a `vi.mock` that nothing imports registers nothing at all, which is
// how these sat inert.
import "./tauri";

/**
 * happy-dom has no 2D context, and every Radial visual is a canvas. Without this
 * the kit's bindings throw on construction and take the component under test with
 * them, which would make a canvas the reason a copy assertion failed.
 *
 * The consequence is deliberate: these tests assert what was handed to a visual,
 * never what it painted. Pixels are the operator's eyes.
 */
const context2d = {
  canvas: null as unknown as HTMLCanvasElement,
  clearRect: vi.fn(),
  fillRect: vi.fn(),
  beginPath: vi.fn(),
  arc: vi.fn(),
  stroke: vi.fn(),
  fill: vi.fn(),
  save: vi.fn(),
  restore: vi.fn(),
  setTransform: vi.fn(),
  scale: vi.fn(),
  translate: vi.fn(),
  fillStyle: "",
  strokeStyle: "",
  lineWidth: 1,
  globalAlpha: 1,
  shadowColor: "",
  shadowBlur: 0,
};

HTMLCanvasElement.prototype.getContext = vi.fn(() => context2d) as never;

afterEach(() => {
  cleanup();
  vi.clearAllMocks();
});
