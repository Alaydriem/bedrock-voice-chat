import assert from "node:assert/strict";
import { describe, it } from "node:test";
import { PositionalSource } from "../core/sources/PositionalSource";
import { ProximityCast } from "../core/sources/ProximityCast";

/**
 * Four surfaces draw this cast, and its tuning is what makes them read as a product that
 * works: people near you, talking, all of the time. Two ways for that to go wrong are
 * invisible in a screenshot and fatal to the point — a voice that falls under the ring's
 * floor vanishes, and a member that drifts past range vanishes with it. Either leaves the
 * screen that sells proximity looking broken.
 */

/** Two minutes at 60fps, which covers every cycle in the tuning. */
const SAMPLES = Array.from({ length: 7200 }, (_, i) => i * 16.7);

const NEARBY = 4;

describe("the proximity cast", () => {
  it("never lets a voice fall silent", () => {
    for (const t of SAMPLES) {
      for (let i = 0; i < ProximityCast.ROSTER.length; i++) {
        const level = ProximityCast.voice(t, i);
        assert.ok(level > 0.03, `member ${i} fell to ${level} at ${t}ms`);
        assert.ok(level <= 1, `member ${i} reached ${level} at ${t}ms`);
      }
    }
  });

  /**
   * The four the introduction and the gate place have to stay audible. Falloff is quadratic,
   * so a member at 0.7 of range contributes under the 0.08 speaking threshold and its row
   * greys out — being "in range" is not enough, they have to stay near.
   */
  it("keeps everyone it places in earshot", () => {
    for (const t of SAMPLES) {
      assert.equal(
        ProximityCast.placements(t, NEARBY).length,
        NEARBY,
        `someone dropped out of earshot at ${t}ms`,
      );
    }
  });

  it("keeps distances inside the range the server enforces", () => {
    for (const t of SAMPLES) {
      for (let i = 0; i < ProximityCast.ROSTER.length; i++) {
        const distance = ProximityCast.distance(i, t);
        assert.ok(distance > 0, `member ${i} arrived at ${distance} m at ${t}ms`);
        assert.ok(
          distance < PositionalSource.RANGE,
          `member ${i} walked to ${distance} m at ${t}ms`,
        );
      }
    }
  });

  // Volume is a ring amplitude, and a bar asks for segments in proportion to it. Past 1 the
  // renderer clamps, so anything above it is a silent loss of range rather than a louder bar.
  it("hands the ring volumes it can use", () => {
    for (const t of SAMPLES) {
      for (const source of ProximityCast.placements(t, ProximityCast.ROSTER.length)) {
        assert.ok(source.volume > 0 && source.volume <= 1, `volume ${source.volume} at ${t}ms`);
      }
    }
  });

  /**
   * Placing fewer members must place the *nearest* ones. Step one shows four because five
   * rows of readout overflow the pane, and the four it shows are the four the readout lists.
   */
  it("places the first members of the roster, in order", () => {
    const placed = ProximityCast.placements(1234, NEARBY);
    const hues = ProximityCast.ROSTER.slice(0, NEARBY).map((member) => member.hue);
    assert.deepEqual(
      placed.map((source) => source.hue),
      hues,
    );
  });

  it("cannot be asked for more members than exist", () => {
    const placed = ProximityCast.placements(1234, 99);
    assert.ok(placed.length <= ProximityCast.ROSTER.length);
  });
});
