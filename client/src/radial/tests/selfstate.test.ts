import assert from "node:assert/strict";
import { describe, it } from "node:test";
import { SelfState } from "../core/controllers/SelfState";

/**
 * The three invariants exist because each is a state people reach by accident and
 * cannot detect from their own screen. They are the reason SelfState is a state machine
 * rather than three booleans on a component.
 */
describe("SelfState", () => {
  it("starts audible", () => {
    const self = new SelfState();
    assert.equal(self.snapshot.muted, false);
    assert.equal(self.snapshot.deafened, false);
    assert.equal(self.transmitting, true);
  });

  it("mutes when you deafen, because hearing nobody while they hear you is a trap", () => {
    const self = new SelfState();
    self.toggleDeafen();
    assert.equal(self.snapshot.deafened, true);
    assert.equal(self.snapshot.muted, true);
    assert.equal(self.transmitting, false);
  });

  it("undeafens when you unmute, so one press gets you back in", () => {
    const self = new SelfState();
    self.toggleDeafen();
    self.toggleMute();
    assert.equal(self.snapshot.deafened, false);
    assert.equal(self.snapshot.muted, false);
    assert.equal(self.transmitting, true);
  });

  it("undeafening on its own leaves you unmuted too", () => {
    const self = new SelfState();
    self.toggleDeafen();
    self.toggleDeafen();
    assert.equal(self.snapshot.deafened, false);
    assert.equal(self.snapshot.muted, false);
  });

  it("treats push-to-talk as a mode of the mic button, not a sibling", () => {
    const self = new SelfState();
    self.setMode("ptt");
    // Not holding already is mute; a separate mute would be a second word for it.
    assert.equal(self.snapshot.muted, false);
    assert.equal(self.transmitting, false);
    self.hold(true);
    assert.equal(self.transmitting, true);
    self.hold(false);
    assert.equal(self.transmitting, false);
  });

  it("clears a stale mute when entering push-to-talk", () => {
    // Otherwise the hold control silently does nothing, which reads as a broken button
    // rather than as a muted mic.
    const self = new SelfState();
    self.toggleMute();
    self.setMode("ptt");
    assert.equal(self.snapshot.muted, false);
    self.hold(true);
    assert.equal(self.transmitting, true);
  });

  it("ignores hold outside push-to-talk", () => {
    const self = new SelfState();
    self.hold(true);
    assert.equal(self.snapshot.holding, false);
  });

  it("does not emit for a hold that changes nothing", () => {
    const self = new SelfState();
    self.setMode("ptt");
    let emissions = 0;
    self.subscribe(() => emissions++);
    const baseline = emissions;
    self.hold(false);
    assert.equal(emissions, baseline);
    self.hold(true);
    assert.equal(emissions, baseline + 1);
  });

  it("times a recording from when it was armed", () => {
    const self = new SelfState();
    assert.equal(self.elapsed(1000), 0);
    self.toggleRecording(1000);
    assert.equal(self.elapsed(4000), 3000);
    self.toggleRecording(4000);
    assert.equal(self.elapsed(9000), 0);
  });

  it("hands every subscriber the same snapshot", () => {
    const self = new SelfState();
    const seen: boolean[] = [];
    self.subscribe((s) => seen.push(s.muted));
    self.subscribe((s) => seen.push(s.muted));
    seen.length = 0;
    self.toggleMute();
    assert.deepEqual(seen, [true, true]);
  });

  it("resets to a clean session", () => {
    const self = new SelfState();
    self.toggleDeafen();
    self.setMode("ptt");
    self.toggleRecording(0);
    self.reset();
    const s = self.snapshot;
    assert.deepEqual(
      { muted: s.muted, deafened: s.deafened, recording: s.recording, mode: s.mode },
      { muted: false, deafened: false, recording: false, mode: "activated" },
    );
  });
});
