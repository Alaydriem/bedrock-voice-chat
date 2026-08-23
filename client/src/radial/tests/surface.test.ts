import assert from "node:assert/strict";
import { describe, it } from "node:test";
import { Surface } from "../core/canvas/Surface";

interface FakeCanvas {
  width: number;
  height: number;
  style: { width: string; height: string };
  getBoundingClientRect(): { width: number; height: number };
  getContext(kind: string): unknown;
  measurements: number;
  box: { width: number; height: number };
  observer: ((entries: unknown[]) => void) | null;
}

/**
 * A canvas that counts how often its layout box is read.
 *
 * The count is the whole point: reading the box is what turns a pending style
 * invalidation into a synchronous reflow, and the preloader writes text into the
 * status line eleven times a second while the ring animates.
 */
function fakeCanvas(width = 320, height = 320): FakeCanvas {
  const canvas: FakeCanvas = {
    width: 0,
    height: 0,
    style: { width: "", height: "" },
    measurements: 0,
    box: { width, height },
    observer: null,
    getBoundingClientRect() {
      canvas.measurements++;
      return { width: canvas.box.width, height: canvas.box.height };
    },
    getContext() {
      return {
        setTransform() {},
        clearRect() {},
      };
    },
  };
  return canvas;
}

/**
 * Stands in for the browser's ResizeObserver, holding the callback so a test can
 * deliver a resize deliberately rather than waiting on a layout that never happens
 * outside a browser.
 */
function withResizeObserver<T>(canvas: FakeCanvas, body: () => T): T {
  const globals = globalThis as unknown as { ResizeObserver?: unknown };
  const previous = globals.ResizeObserver;

  globals.ResizeObserver = class {
    #callback: (entries: unknown[]) => void;
    constructor(callback: (entries: unknown[]) => void) {
      this.#callback = callback;
    }
    observe() {
      canvas.observer = this.#callback;
    }
    disconnect() {
      canvas.observer = null;
    }
  };

  try {
    return body();
  } finally {
    globals.ResizeObserver = previous;
  }
}

function surfaceFor(canvas: FakeCanvas): Surface {
  return new Surface(canvas as unknown as HTMLCanvasElement);
}

describe("Surface.fit", () => {
  it("reports the box it measured", () => {
    const canvas = fakeCanvas(240, 240);
    withResizeObserver(canvas, () => {
      const surface = surfaceFor(canvas);
      assert.equal(surface.fit(), true);
      assert.equal(surface.width, 240);
      assert.equal(surface.height, 240);
    });
  });

  // The fix. Every frame used to read the layout box, so any pending text change
  // upstream became a forced synchronous reflow on the very next frame.
  it("measures once across many frames when the box has not changed", () => {
    const canvas = fakeCanvas();
    withResizeObserver(canvas, () => {
      const surface = surfaceFor(canvas);
      for (let frame = 0; frame < 60; frame++) surface.fit();
      assert.equal(canvas.measurements, 1);
    });
  });

  it("re-measures after the observer reports a resize", () => {
    const canvas = fakeCanvas();
    withResizeObserver(canvas, () => {
      const surface = surfaceFor(canvas);
      surface.fit();
      assert.equal(canvas.measurements, 1);

      canvas.box = { width: 400, height: 400 };
      canvas.observer?.([]);

      assert.equal(surface.fit(), true);
      assert.equal(canvas.measurements, 2);
      assert.equal(surface.width, 400);
    });
  });

  // A canvas inside a hidden pane has no box yet. Caching that answer would leave the
  // surface permanently convinced it can never paint.
  it("keeps asking while the canvas has no box", () => {
    const canvas = fakeCanvas(0, 0);
    withResizeObserver(canvas, () => {
      const surface = surfaceFor(canvas);
      assert.equal(surface.fit(), false);
      assert.equal(surface.fit(), false);
      assert.equal(canvas.measurements, 2);

      canvas.box = { width: 300, height: 300 };
      assert.equal(surface.fit(), true);
      assert.equal(surface.width, 300);
    });
  });

  // Without ResizeObserver there is nothing to invalidate the cache, so measuring every
  // frame is the only correct behaviour.
  it("falls back to measuring every frame when ResizeObserver is absent", () => {
    const canvas = fakeCanvas();
    const globals = globalThis as unknown as { ResizeObserver?: unknown };
    const previous = globals.ResizeObserver;
    delete globals.ResizeObserver;
    try {
      const surface = surfaceFor(canvas);
      surface.fit();
      surface.fit();
      surface.fit();
      assert.equal(canvas.measurements, 3);
    } finally {
      globals.ResizeObserver = previous;
    }
  });

  // `resize` sets the size explicitly, so a later frame must not overwrite it with a
  // stale cached measurement.
  it("adopts an explicit resize without re-measuring", () => {
    const canvas = fakeCanvas();
    withResizeObserver(canvas, () => {
      const surface = surfaceFor(canvas);
      surface.fit();
      surface.resize(500, 500);

      assert.equal(surface.width, 500);
      surface.fit();
      assert.equal(surface.width, 500);
      assert.equal(canvas.measurements, 1);
    });
  });
});
