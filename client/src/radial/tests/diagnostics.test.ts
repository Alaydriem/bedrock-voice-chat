import assert from "node:assert/strict";
import { describe, it } from "node:test";
import { Diagnostics, type DiagnosticsInput } from "../core/controllers/Diagnostics";
import { ScopeBuffer } from "../core/ring/ScopeBuffer";

const healthy: DiagnosticsInput = {
  rtt: 41,
  lossPercent: 0.2,
  jitterMs: 42,
  jitterDrops: 0,
  datagramsIn: 48,
  datagramsOut: 50,
  capturing: 50,
  inputDevice: "Focusrite Scarlett 2i2",
  inputRate: 48000,
  outputDevice: "Sennheiser HD 560S",
  outputRate: 48000,
  quicPort: 443,
  protocol: "1.3.0",
  rangeMetres: 80,
  falloff: "inverse-square",
  server: "bvc.alaydriem.com",
  uptimeSeconds: 2531,
  reconnecting: false,
  muted: false,
  noiseGate: "Open",
  deafened: false,
  pttIdle: false,
  mutedOthers: 0,
  visiblePlayers: 4,
};

/**
 * The verdict is first-failing-check-wins, and the order is the point: things that stop
 * the product working, then things the user did to themselves, then things that merely
 * degrade quality. Someone who opened this panel has a problem, and a summary makes them
 * do the diagnosis themselves.
 */
describe("Diagnostics.verdict", () => {
  it("says everything is fine when it is", () => {
    const { severity, code } = Diagnostics.verdict(healthy);
    assert.equal(severity, "ok");
    assert.equal(code, "fine");
  });

  it("leads with reconnecting over everything else", () => {
    const { severity, code } = Diagnostics.verdict({
      ...healthy,
      reconnecting: true,
      muted: true,
      deafened: true,
      lossPercent: 9,
      inputRate: 44100,
    });
    assert.equal(severity, "bad");
    assert.equal(code, "reconnecting");
  });

  it("reports deafened before muted, because deafen implies mute", () => {
    // Both flags are set whenever you deafen, so reporting "you are muted" would name
    // the symptom rather than the thing the user chose.
    const { severity, code } = Diagnostics.verdict({ ...healthy, deafened: true, muted: true });
    assert.equal(severity, "warn");
    assert.equal(code, "deafened");
  });

  it("calls a muted mic a fault, not a warning", () => {
    const { severity, code } = Diagnostics.verdict({ ...healthy, muted: true });
    assert.equal(severity, "bad");
    assert.equal(code, "muted");
  });

  it("explains an idle push-to-talk before blaming the hardware", () => {
    const { code } = Diagnostics.verdict({ ...healthy, pttIdle: true, inputRate: 44100 });
    assert.equal(code, "ptt-idle");
  });

  // In push-to-talk a shut microphone is the mode at rest, not a fault. Alarming about it
  // states the same fact twice, in coral, whenever nobody is holding the button.
  it("reports push-to-talk before muted, because the mode is why it is muted", () => {
    const { severity, code } = Diagnostics.verdict({ ...healthy, pttIdle: true, muted: true });
    assert.equal(severity, "warn");
    assert.equal(code, "ptt-idle");
  });

  /**
   * The mic closes a beat after the button is released, so `muted` lags `pttIdle`. Ranked
   * below it, the panel cycled fine → push-to-talk → muted in a third of a second every
   * time somebody stopped talking.
   */
  it("says the same thing either side of the release tail", () => {
    const duringTail = Diagnostics.verdict({ ...healthy, pttIdle: true, muted: false });
    const afterTail = Diagnostics.verdict({ ...healthy, pttIdle: true, muted: true });
    assert.deepEqual(duringTail, afterTail);
  });

  // Held, the mic is genuinely open and there is nothing to report.
  it("is quiet while the button is held", () => {
    const { severity } = Diagnostics.verdict({ ...healthy, pttIdle: false, muted: false });
    assert.equal(severity, "ok");
  });

  // A mute nobody asked push-to-talk for is still a fault worth the alarm.
  it("still alarms about a mute outside push-to-talk", () => {
    const { severity, code } = Diagnostics.verdict({ ...healthy, pttIdle: false, muted: true });
    assert.equal(severity, "bad");
    assert.equal(code, "muted");
  });

  it("names the actual sample rate", () => {
    const { severity, code, params } = Diagnostics.verdict({ ...healthy, inputRate: 44100 });
    assert.equal(severity, "warn");
    assert.equal(code, "input-rate");
    assert.equal(params?.kHz, "44.1");
  });

  it("only complains about loss once it is audible", () => {
    assert.equal(Diagnostics.verdict({ ...healthy, lossPercent: 2.9 }).severity, "ok");
    assert.equal(Diagnostics.verdict({ ...healthy, lossPercent: 3.1 }).severity, "warn");
  });

  it("reminds you when you are the one who muted someone", () => {
    // The count travels as a number now. Which plural form it needs is the reader's
    // language's business, and English's two are not enough for Polish.
    const { severity, code, params } = Diagnostics.verdict({ ...healthy, mutedOthers: 1 });
    assert.equal(severity, "warn");
    assert.equal(code, "muted-others");
    assert.equal(params?.count, 1);
    assert.equal(Diagnostics.verdict({ ...healthy, mutedOthers: 2 }).params?.count, 2);
  });
});

describe("Diagnostics.groups", () => {
  it("flags a sample rate mismatch inside the row as well as the verdict", () => {
    const groups = Diagnostics.groups({ ...healthy, inputRate: 44100 });
    const rate = groups[0].rows.find(([key]) => key === "Sample rate");
    assert.ok(rate?.[1].includes("expected 48.0"));
  });

  /**
   * The row was labelled "Noise gate" and reported the mute flag, so it read `open`
   * whenever you were not muted — whether the gate was disabled, bound, open or closed.
   * Someone whose microphone had gone quiet could not tell from it whether the gate was
   * even in the audio path, and `open` invited them to conclude that it was.
   */
  it("says the gate is off rather than open when it is not in the audio path", () => {
    const groups = Diagnostics.groups({ ...healthy, noiseGate: "Disabled" });
    const gate = groups[0].rows.find(([key]) => key === "Noise gate");
    assert.ok(gate?.[1].includes("off"));
    assert.ok(!gate?.[1].includes("open"));
  });

  it("distinguishes a gate that is open from one that is cutting", () => {
    const open = Diagnostics.groups({ ...healthy, noiseGate: "Open" })[0].rows.find(
      ([key]) => key === "Noise gate",
    );
    assert.ok(open?.[1].includes("open"));

    const closed = Diagnostics.groups({ ...healthy, noiseGate: "Closed" })[0].rows.find(
      ([key]) => key === "Noise gate",
    );
    assert.ok(closed?.[1].includes("closed"));
  });

  // Only a gate that is bound and shut can be the reason, so only that one says so.
  it("points at the gate when it is the thing holding the mic shut", () => {
    const closed = Diagnostics.groups({ ...healthy, noiseGate: "Closed" })[0].rows.find(
      ([key]) => key === "Noise gate",
    );
    assert.ok(closed?.[1].includes("←"));

    const disabled = Diagnostics.groups({ ...healthy, noiseGate: "Disabled" })[0].rows.find(
      ([key]) => key === "Noise gate",
    );
    assert.ok(!disabled?.[1].includes("←"));
  });

  // Mute is its own row and its own verdict. Reporting it here said nothing about the gate
  // and hid the one thing this row exists to show.
  it("reports the gate rather than the mute flag", () => {
    const muted = Diagnostics.groups({ ...healthy, muted: true, noiseGate: "Open" })[0].rows.find(
      ([key]) => key === "Noise gate",
    );
    assert.ok(!muted?.[1].includes("muted"));
    assert.ok(muted?.[1].includes("open"));
  });

  it("says so plainly when nothing is going out", () => {
    const groups = Diagnostics.groups({ ...healthy, datagramsOut: 0 });
    const sending = groups[0].rows.find(([key]) => key === "Sending");
    assert.ok(sending?.[1].includes("nothing is going out"));
  });

  /*
   * The sending figure was taken from every datagram this client sends, and position,
   * presence, control and health traffic all leave over the same socket. It therefore read as
   * a healthy microphone on a client capturing nothing at all, which is exactly the reading
   * that sent a real dead-capture report the wrong way. Naming it audio is half the fix; the
   * capture row below is the other half.
   */
  it("names the sending figure as audio rather than as all traffic", () => {
    const sending = Diagnostics.groups(healthy)[0].rows.find(([key]) => key === "Sending");
    assert.ok(sending?.[1].includes("audio datagrams/s"));
  });

  it("accuses the microphone only once capture has actually been measured", () => {
    const unmeasured = Diagnostics.groups({ ...healthy, capturing: null })[0].rows.find(
      ([key]) => key === "Capturing",
    );
    assert.ok(!unmeasured?.[1].includes("stopped"));
    assert.ok(unmeasured?.[1].includes("not measured"));
  });

  it("points at the microphone when the device has stopped delivering", () => {
    const stopped = Diagnostics.groups({ ...healthy, capturing: 0 })[0].rows.find(
      ([key]) => key === "Capturing",
    );
    assert.ok(stopped?.[1].includes("←"));
    assert.ok(stopped?.[1].includes("stopped"));
  });

  /*
   * A dead capture device and a client sending nothing are different faults with different
   * fixes, and the panel has to be able to show one without the other: capture stopping while
   * the uplink keeps moving is the whole signature of the failure this row was added for.
   */
  it("reports capture separately from what reaches the network", () => {
    const rows = Diagnostics.groups({ ...healthy, capturing: 0, datagramsOut: 50 })[0].rows;
    assert.ok(rows.find(([key]) => key === "Capturing")?.[1].includes("stopped"));
    assert.ok(!rows.find(([key]) => key === "Sending")?.[1].includes("nothing is going out"));
  });

  it("reports a live capture rate without an accusation", () => {
    const live = Diagnostics.groups({ ...healthy, capturing: 49.6 })[0].rows.find(
      ([key]) => key === "Capturing",
    );
    assert.equal(live?.[1], "50 frames/s");
  });

  it("marks a non-standard QUIC port as a fallback", () => {
    const groups = Diagnostics.groups({ ...healthy, quicPort: 8443 });
    const port = groups[2].rows.find(([key]) => key === "QUIC port");
    assert.ok(port?.[1].includes("fallback"));
    const standard = Diagnostics.groups(healthy)[2].rows.find(([key]) => key === "QUIC port");
    assert.ok(!standard?.[1].includes("fallback"));
  });

  it("drops the hours segment from a short uptime", () => {
    assert.equal(Diagnostics.duration(65), "01:05");
    assert.equal(Diagnostics.duration(3725), "1:02:05");
  });
});

describe("ScopeBuffer", () => {
  it("ages from the write head, so one bar changes per tick", () => {
    // Ageing from index zero would make the whole ring flicker every second instead of
    // the trace decaying around it.
    const buffer = new ScopeBuffer(4, 0);
    buffer.push(10);
    assert.equal(buffer.age(0), 0);
    assert.equal(buffer.age(3), 1);
    buffer.push(20);
    assert.equal(buffer.age(1), 0);
    assert.equal(buffer.age(0), 1);
  });

  it("wraps rather than growing", () => {
    const buffer = new ScopeBuffer(3, 0);
    for (const v of [1, 2, 3, 4]) buffer.push(v);
    assert.deepEqual([buffer.at(0), buffer.at(1), buffer.at(2)], [4, 2, 3]);
  });

  it("gives the newest sample the most life", () => {
    const buffer = new ScopeBuffer(8, 0);
    buffer.push(50);
    assert.equal(buffer.life(0), 1);
    assert.ok(buffer.life(1) < buffer.life(0));
  });

  it("resets to a plausible trace rather than to zero", () => {
    const buffer = new ScopeBuffer(4, 0);
    buffer.push(90);
    buffer.reset(36);
    assert.equal(buffer.head, 0);
    assert.deepEqual([0, 1, 2, 3].map((i) => buffer.at(i)), [36, 36, 36, 36]);
  });
});
