import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { describe, it } from "node:test";
import { MarkData } from "../core/mark/MarkData";
import { MarkRenderer } from "../core/mark/MarkRenderer";

/**
 * The mark is the whole system, so the thing worth guarding is that the 23 triplets
 * still describe the SVG they were extracted from, and that the amplitude envelope
 * behaves at its boundaries. Neither test needs a canvas.
 */

describe("MarkData against the exported SVG cells", () => {
  /** [column, row, hex] for every filled cell, exported from the logo. */
  const cells: [number, number, string][] = JSON.parse(
    readFileSync(new URL("../../../static/images/cells.json", import.meta.url), "utf8"),
  );

  const spans = new Map<number, { top: number; bottom: number }>();
  for (const [column, row] of cells) {
    const span = spans.get(column);
    if (!span) spans.set(column, { top: row, bottom: row });
    else {
      span.top = Math.min(span.top, row);
      span.bottom = Math.max(span.bottom, row);
    }
  }

  it("has one column per column of the export", () => {
    assert.equal(MarkData.COLS, spans.size);
  });

  it("spans the same 13 rows", () => {
    const rows = cells.map(([, row]) => row);
    assert.equal(Math.min(...rows), 0);
    assert.equal(Math.max(...rows), MarkData.ROWS - 1);
  });

  it("matches the exported vertical span of every column", () => {
    // A mismatch here means the logo and the design system have drifted apart, which
    // is worth knowing before anything else is rebuilt on top of the triplets.
    for (const [index, column] of MarkData.COLUMNS.entries()) {
      const span = spans.get(index);
      assert.ok(span, `column ${index} missing from the export`);
      assert.deepEqual([column[0], column[1]], [span.top, span.bottom], `column ${index}`);
    }
  });

  it("wraps hueAt so any integer is a valid column", () => {
    assert.equal(MarkData.hueAt(0), MarkData.hueAt(MarkData.COLS));
    assert.equal(MarkData.hueAt(-1), MarkData.hueAt(MarkData.COLS - 1));
  });
});

describe("MarkRenderer amplitude", () => {
  it("collapses every column onto the mid row at zero gain", () => {
    for (let c = 0; c < MarkData.COLS; c++) {
      assert.deepEqual([...MarkRenderer.extent(c, 0)], [MarkData.MID, MarkData.MID]);
    }
  });

  it("draws the column's full span at gain 1", () => {
    for (const [c, column] of MarkData.COLUMNS.entries()) {
      assert.deepEqual([...MarkRenderer.extent(c, 1)], [column[0], column[1]]);
    }
  });

  it("never inverts a span at any amplitude", () => {
    for (let c = 0; c < MarkData.COLS; c++) {
      for (let gain = 0; gain <= 1; gain += 0.05) {
        const [top, bottom] = MarkRenderer.extent(c, gain);
        assert.ok(top <= bottom, `column ${c} inverted at gain ${gain}`);
        assert.ok(top >= 0 && bottom < MarkData.ROWS, `column ${c} escaped the grid at gain ${gain}`);
      }
    }
  });

  it("keeps the dance inside the gain it was given", () => {
    // The envelope scales the amplitude; it must never exceed it, or a quiet speaker
    // would draw taller than a loud one.
    for (let c = 0; c < MarkData.COLS; c++) {
      for (let t = 0; t < 20000; t += 137) {
        const v = MarkRenderer.dance(c, t, 0.5);
        assert.ok(v >= 0 && v <= 0.5 + 1e-9, `dance escaped its gain: ${v}`);
      }
    }
  });

  it("holds still under reduced motion", () => {
    assert.equal(MarkRenderer.dance(3, 1234, 0.7, true), 0.7);
    assert.equal(MarkRenderer.dance(3, 9999, 0.7, true), 0.7);
  });
});
