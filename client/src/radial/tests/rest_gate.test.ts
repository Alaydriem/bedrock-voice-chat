import assert from "node:assert/strict";
import { describe, it } from "node:test";
import { RestGate } from "../core/canvas/RestGate";

describe("RestGate", () => {
  /*
   * The frame that draws the resting picture. Suppressing it would leave the canvas showing
   * whatever it held on the way down — a meter frozen partway rather than one at rest, which is
   * the single thing a level meter must never look like.
   */
  it("draws the first frame at rest", () => {
    const gate = new RestGate();

    assert.equal(gate.needsPaint(true), true);
  });

  it("stops drawing once the resting picture is on the canvas", () => {
    const gate = new RestGate();
    gate.needsPaint(true);
    gate.painted(true);

    for (let frame = 0; frame < 60; frame++) {
      assert.equal(gate.needsPaint(true), false, `frame ${frame}`);
    }
  });

  it("draws every frame while something is moving", () => {
    const gate = new RestGate();

    for (let frame = 0; frame < 60; frame++) {
      assert.equal(gate.needsPaint(false), true, `frame ${frame}`);
      gate.painted(false);
    }
  });

  it("resumes immediately when a resting meter is given something to show", () => {
    const gate = new RestGate();
    gate.needsPaint(true);
    gate.painted(true);
    assert.equal(gate.needsPaint(true), false);

    assert.equal(gate.needsPaint(false), true);
  });

  /*
   * A renderer can decline to draw after asking — an offscreen or zero-sized canvas is the
   * usual reason. Counting that as painted would leave the gate believing a resting picture
   * exists on a canvas that has never been drawn on, and it would stay blank for as long as it
   * stayed quiet, which on a roster is indefinitely.
   */
  it("does not count a frame that was asked for but never drawn", () => {
    const gate = new RestGate();

    assert.equal(gate.needsPaint(true), true);
    // Nothing drawn: no `painted` call.
    assert.equal(gate.needsPaint(true), true);

    gate.painted(true);
    assert.equal(gate.needsPaint(true), false);
  });

  /*
   * Nothing about a resting meter's level will change to make it redraw, so a colour change, a
   * resize or a new source has to say so explicitly or the stale picture stands indefinitely.
   */
  it("redraws once after something the pixels cannot show has changed", () => {
    const gate = new RestGate();
    gate.needsPaint(true);
    gate.painted(true);
    assert.equal(gate.needsPaint(true), false);

    gate.invalidate();
    assert.equal(gate.needsPaint(true), true);
    gate.painted(true);
    assert.equal(gate.needsPaint(true), false);
  });
});
