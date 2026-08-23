import assert from "node:assert/strict";
import { describe, it } from "node:test";
import { CoverDrag } from "../core/controllers/CoverDrag.ts";

describe("CoverDrag", () => {
  it("treats a short pull as a tap rather than a drag", () => {
    assert.equal(CoverDrag.isDrag(4), false);
    assert.equal(CoverDrag.isDrag(CoverDrag.SLOP), true);
  });

  // Upward has nothing to reveal — the cover is already at rest against its peek — so it
  // clamps rather than rubber-bands, which would suggest there is more above.
  it("does not move upward", () => {
    assert.equal(CoverDrag.offset(-80), 0);
    assert.equal(CoverDrag.offset(0), 0);
  });

  it("follows the finger downward", () => {
    assert.equal(CoverDrag.offset(60), 60);
  });

  it("dismisses only past the threshold", () => {
    assert.equal(CoverDrag.dismisses(CoverDrag.DISMISS - 1), false);
    assert.equal(CoverDrag.dismisses(CoverDrag.DISMISS), true);
  });

  // The rule that makes "drag from anywhere" survivable. A finger pulling down halfway
  // through a settings pane means scroll up, not dismiss.
  it("yields to scrollable content that is not at its top", () => {
    assert.equal(CoverDrag.canStart(0), true);
    assert.equal(CoverDrag.canStart(1), false);
    assert.equal(CoverDrag.canStart(400), false);
  });

  // iOS reports a negative scrollTop mid-overscroll. Treating that as "not at the top"
  // would make the gesture fail exactly when the content is furthest past its top.
  it("still starts when the content has overscrolled past its top", () => {
    assert.equal(CoverDrag.canStart(-12), true);
  });

  // A drag released short of the threshold has to be distinguishable from one that was
  // never a drag: both return the cover to rest, but only the first consumed the gesture.
  it("separates a released short drag from a tap", () => {
    const travel = CoverDrag.SLOP + 1;
    assert.equal(CoverDrag.isDrag(travel), true);
    assert.equal(CoverDrag.dismisses(CoverDrag.offset(travel)), false);
  });
});
