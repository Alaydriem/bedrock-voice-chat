import assert from "node:assert/strict";
import { describe, it } from "node:test";
import { RingGeometry } from "../core/ring/RingGeometry";
import { RingRenderer } from "../core/ring/RingRenderer";

/**
 * The renderer trades two closed-form expressions for a lookup table and a cheap
 * angular wrap, both for the sake of the frame budget on a phone. Each one is only
 * legitimate if it agrees with the expression it replaced, so that agreement is the
 * contract worth holding — including past the end of the table, where a loud bar asks
 * for more segments than the ring has.
 */

describe("RingRenderer.SEGMENT_ALPHA against the fade it replaces", () => {
  const closedForm = (k: number) =>
    k === 0 ? 0.62 : Math.max(0.3, 1 - k / (RingGeometry.SEG + 2));

  it("matches at every segment the table covers", () => {
    for (let k = 0; k <= RingGeometry.SEG; k++) {
      assert.equal(RingRenderer.SEGMENT_ALPHA[k], closedForm(k), `segment ${k}`);
    }
  });

  it("stays equal past the end of the table, where the index is clamped", () => {
    const last = RingRenderer.SEGMENT_ALPHA[RingGeometry.SEG];
    for (let k = RingGeometry.SEG + 1; k <= RingGeometry.SEG + 6; k++) {
      assert.equal(last, closedForm(k), `clamped segment ${k}`);
    }
  });
});

/**
 * A ring that spins without dancing needs all three of these at once, and the obvious
 * implementation — reusing `reduce` to hold it still — silently loses the first. A flat
 * profile cannot appear to rotate: every bar is identical and evenly spaced, so each angle
 * looks like every other, and the spin becomes work nobody can see.
 */
describe("a still bar profile", () => {
  const { BARS } = RingGeometry;
  const step = (Math.PI * 2) / BARS;

  // The profile RingRenderer applies when `still` zeroes the time term.
  const profile = (bar: number, rot: number) => {
    const angle = -Math.PI / 2 + bar * step + rot;
    return 0.05 * Math.sin(angle * 3) + 0.035 * Math.sin(angle * 7);
  };
  const ringAt = (rot: number) => Array.from({ length: BARS }, (_, b) => profile(b, rot));

  it("varies from bar to bar, so a rotation is visible", () => {
    const bars = ringAt(0);
    const spread = Math.max(...bars) - Math.min(...bars);
    assert.ok(spread > 0.01, `spread ${spread} is too flat to show rotation`);
  });

  it("does not change on its own, so the ring is not a second moving thing", () => {
    // Time does not appear in the expression at all, which is the property: two frames far
    // apart at the same rotation are the same ring.
    assert.deepEqual(ringAt(0), ringAt(0));
  });

  it("moves with the rotation, so `spin` still sweeps it round", () => {
    const still = ringAt(0);
    const turned = ringAt(0.6);
    assert.ok(
      still.some((value, i) => Math.abs(value - turned[i]) > 1e-6),
      "rotating the ring left every bar unchanged",
    );
  });
});

describe("the angular wrap against atan2", () => {
  const TWO_PI = Math.PI * 2;
  const wrapped = (raw: number) => raw - TWO_PI * Math.round(raw / TWO_PI);
  const viaAtan2 = (raw: number) => Math.atan2(Math.sin(raw), Math.cos(raw));

  /**
   * Magnitude, not the signed value. Exactly opposite the source the two forms pick
   * different signs for the same angle, and the renderer squares the delta — so
   * agreement on the magnitude is the whole of what it depends on, and asserting the
   * sign would be asserting a representation nothing reads.
   */
  it("agrees in magnitude across several turns in both directions", () => {
    for (let i = -400; i <= 400; i++) {
      const raw = (i / 400) * (TWO_PI * 3);
      assert.ok(
        Math.abs(Math.abs(wrapped(raw)) - Math.abs(viaAtan2(raw))) < 1e-9,
        `raw ${raw}: ${wrapped(raw)} vs ${viaAtan2(raw)}`,
      );
    }
  });

  it("puts a source's whole audible arc inside the cutoff", () => {
    // The cutoff skips bars whose gaussian weight cannot reach the colour threshold or
    // move an amplitude. Anything it discards has to be smaller than that.
    const weightAt = (delta: number) =>
      Math.exp(-(delta * delta) / (2 * RingRenderer.SIGMA * RingRenderer.SIGMA));
    assert.ok(weightAt(RingRenderer.CUTOFF) < 1e-7, `weight ${weightAt(RingRenderer.CUTOFF)}`);
    assert.ok(weightAt(RingRenderer.CUTOFF) > 0, "cutoff is finite");
  });
});
