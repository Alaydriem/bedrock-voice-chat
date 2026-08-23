import assert from "node:assert/strict";
import { describe, it } from "node:test";
import { RingBinding } from "../bindings/RingBinding";
import { RingGeometry } from "../core/ring/RingGeometry";

/**
 * A severed ring says "this cannot be completed", and it only says it if the removed bars
 * are one contiguous run with the rest of the circle intact. Both failure modes are
 * invisible in a screenshot and fatal to the meaning: a window that wraps incorrectly
 * removes two arcs and reads as a dashed ring, and one that is not clamped can take the
 * whole circle and leave nothing at all.
 *
 * `RingBinding.cuts` is the observable form of the gating the renderer applies per bar.
 */

const { BARS } = RingGeometry;
const TWO_PI = Math.PI * 2;

/** True when the bars form a single run around the circle, counting the wrap. */
function contiguous(bars: readonly number[]): boolean {
  if (bars.length <= 1) return true;
  const gaps = bars.filter((bar, i) => i > 0 && bar !== bars[i - 1] + 1).length;
  const wraps = bars[0] === 0 && bars[bars.length - 1] === BARS - 1 ? 1 : 0;
  return gaps <= wraps;
}

describe("the window a cut removes", () => {
  it("is one run wherever it is placed, including across the ring's zero angle", () => {
    for (let i = 0; i < 48; i++) {
      const centre = -Math.PI / 2 + (i / 48) * TWO_PI;
      const bars = RingBinding.cuts([centre, 0.46]);
      assert.ok(bars.length > 0, `centre ${centre} removed nothing`);
      assert.ok(contiguous(bars), `centre ${centre} removed ${bars.length} bars in pieces`);
    }
  });

  it("leaves the rest of the ring standing", () => {
    const bars = RingBinding.cuts([-Math.PI / 2 + 2.28, 0.46]);
    assert.ok(bars.length < BARS, "the cut took the whole ring");
    // Two ends to flare needs at least two surviving bars, and a gap that reads as a gap
    // needs the survivors to dominate.
    assert.ok(BARS - bars.length > BARS / 2, `only ${BARS - bars.length} bars survived`);
  });

  it("grows with the half-width and never shrinks", () => {
    const centre = -Math.PI / 2 + 2.28;
    let previous = 0;
    for (const half of [0.1, 0.2, 0.46, 0.8, 1.2]) {
      const count = RingBinding.cuts([centre, half]).length;
      assert.ok(count >= previous, `half ${half} removed ${count}, fewer than ${previous}`);
      previous = count;
    }
  });

  it("removes nothing at zero width, so an absent cut cannot dim the ring", () => {
    assert.deepEqual(RingBinding.cuts([-Math.PI / 2, 0]), []);
  });
});
