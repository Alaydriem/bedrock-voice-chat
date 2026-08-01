import assert from "node:assert/strict";
import { describe, it } from "node:test";
import { IntroEnvelope } from "../core/intro/IntroEnvelope";
import { INTRO_PHASES, IntroMarks } from "../core/intro/IntroPhases";

const AT = IntroMarks.AT;

describe("IntroEnvelope", () => {
  const flat = new IntroEnvelope("flat");
  const dancing = new IntroEnvelope("dance", 0.32);

  it("holds the mark flat until the blocks take their colours", () => {
    assert.equal(flat.gain(0), 0);
    assert.equal(flat.gain(AT.scan), 0);
    assert.equal(flat.gain(AT.charge - 0.001), 0);
  });

  it("rises to full amplitude through the wave", () => {
    assert.ok(flat.gain(AT.charge + 0.2) > 0);
    assert.ok(Math.abs(flat.gain(AT.wave) - 1) < 0.05);
  });

  it("eases back to make room for the ring flying in", () => {
    // Measured just inside the phase: at the boundary itself the first beat is already
    // firing, which is the point — the mark hits as the bars land.
    const justBefore = flat.gain(AT.implode - 0.001);
    assert.ok(justBefore < flat.gain(AT.wave));
    assert.ok(Math.abs(justBefore - 0.55) < 0.02, `expected ~0.55, got ${justBefore}`);
  });

  it("hits full amplitude at the moment the bars land", () => {
    assert.ok(Math.abs(flat.gain(AT.implode) - 1) < 1e-9);
  });

  it("stays inside 0 and 1 across the whole timeline", () => {
    for (let t = 0; t <= AT.total + 4; t += 0.01) {
      const g = flat.gain(t);
      assert.ok(g >= -1e-9 && g <= 1 + 1e-9, `gain ${g} at t=${t.toFixed(2)}`);
    }
  });

  it("lands on the mid row when the end state is flat", () => {
    assert.ok(flat.gain(AT.settle) < 0.001);
  });

  it("lands on the idle amplitude when it keeps dancing", () => {
    // This is the loader's resting state: still alive, still saying something is
    // happening, at a level that does not compete with the content around it.
    assert.ok(Math.abs(dancing.gain(AT.settle) - 0.32) < 0.001);
  });

  it("beats three times through the pulse", () => {
    // A sawtooth: each hit starts at full and decays to nothing, then snaps back. Three
    // hits rather than one long swell is what makes it read as a pulse.
    const third = INTRO_PHASES.pulse / 3;
    for (const hit of [0, 1, 2]) {
      const onset = AT.implode + hit * third;
      assert.ok(flat.beat(onset) > 0.99, `hit ${hit} did not start at full`);
      assert.ok(flat.beat(onset + third * 0.99) < 0.01, `hit ${hit} did not decay`);
    }
  });

  it("counts exactly three snaps back to full", () => {
    let snaps = 0;
    let previous = flat.beat(AT.implode - 0.001);
    for (let t = AT.implode; t < AT.pulse; t += 0.001) {
      const v = flat.beat(t);
      // A rise of more than half in one millisecond is the sawtooth resetting.
      if (v - previous > 0.5) snaps++;
      previous = v;
    }
    assert.equal(snaps, 3);
  });

  it("is silent before the ring lands", () => {
    assert.equal(flat.beat(0), 0);
    assert.equal(flat.beat(AT.wave), 0);
  });

  it("collapses from the idle amplitude to nothing", () => {
    assert.equal(dancing.collapseGain(0), 0.32);
    assert.equal(dancing.collapseGain(1), 0);
    assert.ok(dancing.collapseGain(0.5) < 0.32 && dancing.collapseGain(0.5) > 0);
  });

  it("clamps a collapse that overshoots", () => {
    assert.equal(dancing.collapseGain(1.4), 0);
    assert.equal(dancing.collapseGain(-0.2), 0.32);
  });
});

describe("IntroMarks", () => {
  it("accumulates the phase durations in order", () => {
    assert.equal(AT.scan, INTRO_PHASES.scan);
    assert.equal(AT.charge, AT.scan + INTRO_PHASES.charge);
    assert.equal(AT.settle, AT.pulse + INTRO_PHASES.settle);
    assert.equal(AT.total, AT.settle);
  });
});
