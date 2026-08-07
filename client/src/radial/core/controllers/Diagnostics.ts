export type Severity = "ok" | "warn" | "bad";

/**
 * Whether the noise gate is in the audio path, and what it is doing there.
 *
 * Three states, because a boolean cannot separate the two that matter to someone whose
 * microphone has gone quiet: a gate that is switched off passes everything and a gate that
 * is open passes everything, so "is audio getting through" reads the same for both.
 */
export type NoiseGateStatus = "Disabled" | "Open" | "Closed";

export interface DiagnosticsInput {
  /** Round-trip time in milliseconds. */
  rtt: number;
  lossPercent: number;
  jitterMs: number;
  jitterDrops: number;
  datagramsIn: number;
  /**
   * Audio frames handed to the network each second.
   *
   * Audio only. Taken from the count of every datagram this client sends it read as a healthy
   * microphone on a client capturing nothing, because position, presence, control and health
   * traffic all leave over the same socket and keep that number in the dozens on their own.
   */
  datagramsOut: number;
  /**
   * Frames arriving from the capture device each second, or null before one interval has
   * been measured.
   *
   * Upstream of the gate, the encoder and the network, so it is the only row that separates a
   * microphone that stopped from audio that stopped getting through. Null rather than zero
   * while unmeasured: zero is an accusation.
   */
  capturing: number | null;
  inputDevice: string;
  inputRate: number;
  outputDevice: string;
  outputRate: number;
  quicPort: number;
  protocol: string;
  rangeMetres: number;
  falloff: string;
  server: string;
  uptimeSeconds: number;
  reconnecting: boolean;
  /** Attempt number, when reconnecting. */
  attempt?: number;
  /**
   * Sending while nothing comes back.
   *
   * A live connection always produces acknowledgements, so silence in one direction only is
   * the signature of a server that has stopped processing this client — path-budget
   * exhaustion being the known cause. Nothing else on this panel reveals it: the microphone
   * is fine, the link reads as up, and every other number looks healthy.
   */
  stalled?: boolean;
  /**
   * How much of what was heard had to be reconstructed rather than decoded.
   *
   * Not loss, and never labelled as such — it is what a listener actually experienced, which
   * makes it the most meaningful figure here.
   */
  concealmentPercent?: number;
  muted: boolean;
  /**
   * What the noise gate is doing, read from the flag the capture path itself consults.
   *
   * Spelled out here rather than imported, because the kit does not depend on the app's
   * generated bindings. `DiagnosticsView` assigns the binding straight into this, so a
   * variant added on the Rust side fails to typecheck there rather than drifting quietly.
   */
  noiseGate: NoiseGateStatus;
  deafened: boolean;
  /** Push-to-talk on but not currently held. */
  pttIdle: boolean;
  /** How many other players this client has muted, and out of how many. */
  mutedOthers: number;
  visiblePlayers: number;
}

export interface KvGroup {
  title: string;
  rows: readonly (readonly [string, string])[];
}

/**
 * What the status panel says, and why.
 *
 * `verdict` is first-failing-check-wins rather than a summary. Someone who opens this
 * panel has a problem, and a wall of green numbers with one bad one in the middle makes
 * them do the diagnosis themselves. The order is deliberate: things that stop the
 * product working entirely, then things the user did to themselves, then things that
 * degrade quality.
 *
 * `groups` returns data, not markup, so the panel can write values in place instead of
 * rebuilding its DOM once a second — which reflows the panel and reads as flicker.
 */
export class Diagnostics {
  static readonly EXPECTED_RATE = 48000;

  static verdict(d: DiagnosticsInput): readonly [Severity, string] {
    if (d.reconnecting) {
      const attempt = d.attempt ? ` — attempt ${d.attempt}` : "";
      return ["bad", `Reconnecting${attempt}. Nobody can hear you right now.`];
    }
    // Above loss, because it outranks it: your microphone is fine, the link reads as up, and
    // nothing you say is arriving. Every other number on this panel looks healthy.
    if (d.stalled) {
      return ["bad", "Your audio is not reaching the server. Try reconnecting."];
    }
    if (d.deafened) return ["warn", "You are deafened. You cannot hear anyone."];
    // Above mute, because in push-to-talk mute is not a fault — it is the mode at rest, and
    // the two are the same fact told twice. Below it, a released button read as "You are
    // muted. Nobody can hear you." in coral, and the release tail made the panel cycle
    // through three verdicts in a third of a second: fine while held, push-to-talk for the
    // tail, then muted. Only a mute that is not push-to-talk's doing is worth alarm.
    if (d.pttIdle) return ["warn", "Push-to-talk is on. Hold the mic button to speak."];
    if (d.muted) return ["bad", "You are muted. Nobody can hear you."];
    if (d.inputRate !== Diagnostics.EXPECTED_RATE) {
      return [
        "warn",
        `Your input device is running at ${(d.inputRate / 1000).toFixed(1)} kHz. BVC expects 48 kHz.`,
      ];
    }
    // Above loss, because this is what somebody actually heard. Loss is a cause; concealment
    // is the symptom they came to the panel about.
    //
    // Attributed to one speaker rather than to the session, because it is a maximum across
    // everybody audible. Phrased as "what you heard" it accused the whole link of something one
    // person's uplink was doing, and sent the reader looking for a fault at this end.
    if ((d.concealmentPercent ?? 0) > 5) {
      return [
        "warn",
        `${d.concealmentPercent}% of the worst speaker's audio had to be reconstructed. They will sound rough.`,
      ];
    }
    if (d.lossPercent > 3) {
      return ["warn", `Packet loss is ${d.lossPercent}%. Audio will break up.`];
    }
    if (d.mutedOthers > 0) {
      const plural = d.mutedOthers === 1 ? "player is" : "players are";
      return ["warn", `${d.mutedOthers} ${plural} muted by you.`];
    }
    return ["ok", "Everything looks fine."];
  }

  /**
   * The gate row.
   *
   * Three states rather than two, because "off" and "open" both pass audio and this row
   * used to print the mute flag instead — so it read `open` whether the gate was bound or
   * not, and someone whose mic had gone quiet had no way to rule it in or out. Only a gate
   * that is on and shut can be the cause, so only that one gets the arrow.
   */
  static gate(status: NoiseGateStatus): string {
    switch (status) {
      case "Disabled":
        return "off (not in the audio path)";
      case "Open":
        return "on, open (passing audio)";
      case "Closed":
        return "on, closed  ← this is cutting your mic";
    }
  }

  /**
   * The capture row.
   *
   * The row above it reports what left for the network, and the two only look alike while
   * everything works. A device that stops delivering leaves this at zero while the gate still
   * reads open and the sending figure still moves, which is the shape of the fault that was
   * previously impossible to tell apart from a quiet room.
   */
  static capture(framesPerSecond: number | null): string {
    if (framesPerSecond === null) return "not measured yet";
    if (framesPerSecond === 0) return "0 frames/s  ← your microphone has stopped";
    return `${Math.round(framesPerSecond)} frames/s`;
  }

  static groups(d: DiagnosticsInput): KvGroup[] {
    const rate = (hz: number) => `${(hz / 1000).toFixed(1)} kHz`;
    return [
      {
        title: "Your mic",
        rows: [
          ["Device", d.inputDevice],
          [
            "Sample rate",
            rate(d.inputRate) + (d.inputRate !== Diagnostics.EXPECTED_RATE ? "  ← expected 48.0" : ""),
          ],
          ["Noise gate", Diagnostics.gate(d.noiseGate)],
          ["Capturing", Diagnostics.capture(d.capturing)],
          [
            "Sending",
            `${d.datagramsOut} audio datagrams/s` +
              (d.datagramsOut === 0 ? "  ← nothing is going out" : ""),
          ],
        ],
      },
      {
        title: "What you hear",
        rows: [
          ["Device", d.outputDevice],
          ["Sample rate", rate(d.outputRate)],
          ["Receiving", `${d.datagramsIn} datagrams/s`],
          [
            "Muted by you",
            // The denominator is dropped when it is not known rather than printed as zero:
            // "1 of 0" is arithmetic nobody can act on, and the two figures come from different
            // places — the count from this client's own mixer, the population from the position
            // feed, which answers with nothing until it is connected.
            d.mutedOthers
              ? d.visiblePlayers > 0
                ? `${d.mutedOthers} of ${d.visiblePlayers}`
                : `${d.mutedOthers}`
              : "none",
          ],
        ],
      },
      {
        title: "Link",
        rows: [
          [
            "State",
            d.reconnecting
              ? `reconnecting (${d.attempt ?? 1})`
              : d.stalled
                ? `connected  ${Diagnostics.duration(d.uptimeSeconds)}  ← stalled`
                : `connected  ${Diagnostics.duration(d.uptimeSeconds)}`,
          ],
          ["Round trip", `${d.rtt} ms`],
          ["Packet loss", `${d.lossPercent} %`],
          ["Jitter buffer", `${d.jitterMs} ms  /  ${d.jitterDrops} drops`],
          ["QUIC port", `${d.quicPort}${d.quicPort !== 443 ? "  (fallback)" : ""}`],
        ],
      },
      {
        title: "Session",
        rows: [
          ["Server", d.server],
          ["Protocol", d.protocol],
          ["Proximity range", `${d.rangeMetres} m`],
          ["Falloff", d.falloff],
        ],
      },
    ];
  }

  static duration(seconds: number): string {
    const h = Math.floor(seconds / 3600);
    const m = Math.floor((seconds % 3600) / 60);
    const s = Math.floor(seconds % 60);
    const mm = String(m).padStart(2, "0");
    const ss = String(s).padStart(2, "0");
    return h ? `${h}:${mm}:${ss}` : `${mm}:${ss}`;
  }
}
