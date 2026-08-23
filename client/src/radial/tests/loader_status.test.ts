import assert from "node:assert/strict";
import { describe, it } from "node:test";
import { LoaderStatus } from "../core/intro/LoaderStatus";

const PHRASES = ["Reaching your server…", "Checking your permissions…", "Almost there…"];

describe("LoaderStatus", () => {
  const status = new LoaderStatus({ phrases: PHRASES, slowAfterSeconds: 4 });

  it("stays hidden until the wait is long enough to be worth explaining", () => {
    assert.equal(status.at(0).visible, false);
    assert.equal(status.at(3.9).visible, false);
  });

  it("appears once the threshold is crossed", () => {
    assert.equal(status.at(4).visible, true);
    assert.equal(status.at(30).visible, true);
  });

  // Sampled at the middle of each interval, not its boundary: 0.12 has no exact
  // binary representation, so 0.12 * 6 / 0.12 floors to 5 and a boundary sample
  // fails for reasons unrelated to the code under test.
  it("advances the phrase on its own cadence, not the frame's", () => {
    assert.notEqual(status.at(4 + 0.8).phrase, status.at(4 + 1.6 + 0.8).phrase);
  });

  it("cycles phrases rather than running off the end", () => {
    assert.equal(status.at(4 + 1.6 * PHRASES.length + 0.8).phrase, PHRASES[0]);
  });

  it("cycles the braille glyph", () => {
    const glyphs = new Set<string>();
    for (let i = 0; i < LoaderStatus.BRAILLE_FRAMES.length; i++) {
      glyphs.add(status.at(4 + (i + 0.5) * 0.12).glyph);
    }
    assert.equal(glyphs.size, LoaderStatus.BRAILLE_FRAMES.length);
  });

  // A loader with nothing to say must not render an empty status line.
  it("is never visible without phrases", () => {
    assert.equal(new LoaderStatus({ phrases: [] }).at(60).visible, false);
  });
});
