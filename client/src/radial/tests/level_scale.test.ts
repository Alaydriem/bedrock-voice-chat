import assert from "node:assert/strict";
import { describe, it } from "node:test";
import { LevelScale } from "../core/sources/LevelScale";

/**
 * The numbers are the point. A linear RMS meter reads flat for every voice that is not
 * against the capsule, which is the defect this replaced, so the guard has to be on where
 * ordinary speech actually lands rather than on the shape of the curve.
 *
 * Reference RMS values for a desk microphone: a quiet room sits near 0.002, conversational
 * speech spans roughly 0.02 to 0.08, and 0.25 is loud enough that nobody would expect
 * headroom above it.
 */

describe("LevelScale.fromRms", () => {
  it("reads silence as nothing", () => {
    assert.equal(LevelScale.fromRms(0), 0);
  });

  it("treats a gated or invalid frame as silence rather than -Infinity", () => {
    // log10 of zero is -Infinity and a gated frame arrives as exactly 0, so the guard is
    // load-bearing: without it the meter takes a NaN and stops drawing entirely.
    assert.equal(LevelScale.fromRms(-0.1), 0);
    assert.equal(LevelScale.fromRms(Number.NaN), 0);
  });

  it("leaves a quiet room near the floor", () => {
    assert.ok(LevelScale.fromRms(0.002) < 0.1, `got ${LevelScale.fromRms(0.002)}`);
  });

  /**
   * The regression this exists for. A conversational voice has to be unmistakable, not a
   * flicker at the bottom of the meter — under the linear mapping 0.02 RMS produced 0.02,
   * two percent of the range.
   */
  it("puts a conversational voice well up the range", () => {
    const quietSpeech = LevelScale.fromRms(0.02);
    const normalSpeech = LevelScale.fromRms(0.05);
    assert.ok(quietSpeech > 0.45, `0.02 RMS gave ${quietSpeech}`);
    assert.ok(normalSpeech > 0.7, `0.05 RMS gave ${normalSpeech}`);
  });

  it("reaches full on a firm voice, without needing to clip", () => {
    assert.equal(LevelScale.fromRms(0.2), 1);
    assert.equal(LevelScale.fromRms(1), 1);
  });

  it("never leaves the unit range", () => {
    for (const rms of [0, 1e-9, 0.001, 0.01, 0.1, 0.5, 1, 4]) {
      const level = LevelScale.fromRms(rms);
      assert.ok(level >= 0 && level <= 1, `${rms} gave ${level}`);
    }
  });

  it("rises with the input", () => {
    let previous = -1;
    for (const rms of [0.001, 0.005, 0.01, 0.02, 0.05, 0.1]) {
      const level = LevelScale.fromRms(rms);
      assert.ok(level > previous, `${rms} gave ${level}, not above ${previous}`);
      previous = level;
    }
  });
});
