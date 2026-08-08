export type Severity = "ok" | "warn" | "bad";

/**
 * Every word the diagnostics panel says that is not a number or a unit.
 *
 * The kit cannot reach the application's translation surface — it runs under `node --test`
 * with no bundler, and the surface is built on runes the type stripper cannot evaluate — so
 * the words arrive as an argument instead. `DIAGNOSTICS_EN` is the default, which keeps the
 * reference gallery and the kit's own tests working with no caller changes.
 */
export interface DiagnosticsLabels {
  yourMic: string;
  whatYouHear: string;
  link: string;
  session: string;

  device: string;
  sampleRate: string;
  noiseGate: string;
  capturing: string;
  sending: string;
  receiving: string;
  mutedByYou: string;
  state: string;
  roundTrip: string;
  packetLoss: string;
  jitterBuffer: string;
  quicPort: string;
  server: string;
  protocol: string;
  proximityRange: string;
  falloff: string;

  gateOff: string;
  gateOpen: string;
  gateClosed: string;
  notMeasuredYet: string;
  micStopped: string;
  nothingGoingOut: string;
  expectedRate: string;
  none: string;
  reconnecting: string;
  connected: string;
  stalled: string;
  fallbackPort: string;
  drops: string;
}

export const DIAGNOSTICS_EN: DiagnosticsLabels = {
  yourMic: "Your mic",
  whatYouHear: "What you hear",
  link: "Link",
  session: "Session",

  device: "Device",
  sampleRate: "Sample rate",
  noiseGate: "Noise gate",
  capturing: "Capturing",
  sending: "Sending",
  receiving: "Receiving",
  mutedByYou: "Muted by you",
  state: "State",
  roundTrip: "Round trip",
  packetLoss: "Packet loss",
  jitterBuffer: "Jitter buffer",
  quicPort: "QUIC port",
  server: "Server",
  protocol: "Protocol",
  proximityRange: "Proximity range",
  falloff: "Falloff",

  gateOff: "off (not in the audio path)",
  gateOpen: "on, open (passing audio)",
  gateClosed: "on, closed  ← this is cutting your mic",
  notMeasuredYet: "not measured yet",
  micStopped: "← your microphone has stopped",
  nothingGoingOut: "← nothing is going out",
  expectedRate: "← expected 48.0",
  none: "none",
  reconnecting: "reconnecting",
  connected: "connected",
  stalled: "← stalled",
  fallbackPort: "(fallback)",
  drops: "drops",
};

/** Which verdict fired. The kit decides this; what it reads is decided elsewhere. */
export type VerdictCode =
  | "reconnecting"
  | "stalled"
  | "deafened"
  | "ptt-idle"
  | "muted"
  | "input-rate"
  | "concealment"
  | "loss"
  | "muted-others"
  | "fine";

/**
 * A verdict as a code and its numbers, not as a sentence.
 *
 * The kit runs framework-free, under `node --test` with no bundler, so it cannot reach the
 * application's translation surface — and it should not: deciding which check failed is its
 * job, and wording that decision is not. Rendering lives in `DiagnosticsCopy`.
 */
export interface Verdict {
  severity: Severity;
  code: VerdictCode;
  params?: Readonly<Record<string, string | number>>;
}

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

  static verdict(d: DiagnosticsInput): Verdict {
    if (d.reconnecting) {
      return { severity: "bad", code: "reconnecting", params: { attempt: d.attempt ?? 0 } };
    }
    // Above loss, because it outranks it: your microphone is fine, the link reads as up, and
    // nothing you say is arriving. Every other number on this panel looks healthy.
    if (d.stalled) {
      return { severity: "bad", code: "stalled" };
    }
    if (d.deafened) return { severity: "warn", code: "deafened" };
    // Above mute, because in push-to-talk mute is not a fault — it is the mode at rest, and
    // the two are the same fact told twice. Below it, a released button read as "You are
    // muted. Nobody can hear you." in coral, and the release tail made the panel cycle
    // through three verdicts in a third of a second: fine while held, push-to-talk for the
    // tail, then muted. Only a mute that is not push-to-talk's doing is worth alarm.
    if (d.pttIdle) return { severity: "warn", code: "ptt-idle" };
    if (d.muted) return { severity: "bad", code: "muted" };
    if (d.inputRate !== Diagnostics.EXPECTED_RATE) {
      return {
        severity: "warn",
        code: "input-rate",
        params: { kHz: (d.inputRate / 1000).toFixed(1) },
      };
    }
    // Above loss, because this is what somebody actually heard. Loss is a cause; concealment
    // is the symptom they came to the panel about.
    //
    // Attributed to one speaker rather than to the session, because it is a maximum across
    // everybody audible. Phrased as "what you heard" it accused the whole link of something one
    // person's uplink was doing, and sent the reader looking for a fault at this end.
    if ((d.concealmentPercent ?? 0) > 5) {
      return {
        severity: "warn",
        code: "concealment",
        params: { percent: d.concealmentPercent ?? 0 },
      };
    }
    if (d.lossPercent > 3) {
      return { severity: "warn", code: "loss", params: { percent: d.lossPercent } };
    }
    if (d.mutedOthers > 0) {
      // The count travels as a number rather than as a finished phrase. Choosing between
      // "player is" and "players are" here would bake English's two-form plural into the
      // kit, which is wrong in every language that has more than two.
      return { severity: "warn", code: "muted-others", params: { count: d.mutedOthers } };
    }
    return { severity: "ok", code: "fine" };
  }

  /**
   * The gate row.
   *
   * Three states rather than two, because "off" and "open" both pass audio and this row
   * used to print the mute flag instead — so it read `open` whether the gate was bound or
   * not, and someone whose mic had gone quiet had no way to rule it in or out. Only a gate
   * that is on and shut can be the cause, so only that one gets the arrow.
   */
  static gate(status: NoiseGateStatus, labels: DiagnosticsLabels = DIAGNOSTICS_EN): string {
    switch (status) {
      case "Disabled":
        return labels.gateOff;
      case "Open":
        return labels.gateOpen;
      case "Closed":
        return labels.gateClosed;
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
  static capture(
    framesPerSecond: number | null,
    labels: DiagnosticsLabels = DIAGNOSTICS_EN,
  ): string {
    if (framesPerSecond === null) return labels.notMeasuredYet;
    if (framesPerSecond === 0) return `0 frames/s  ${labels.micStopped}`;
    return `${Math.round(framesPerSecond)} frames/s`;
  }

  static groups(d: DiagnosticsInput, labels: DiagnosticsLabels = DIAGNOSTICS_EN): KvGroup[] {
    const rate = (hz: number) => `${(hz / 1000).toFixed(1)} kHz`;
    return [
      {
        title: labels.yourMic,
        rows: [
          [labels.device, d.inputDevice],
          [
            labels.sampleRate,
            rate(d.inputRate) +
              (d.inputRate !== Diagnostics.EXPECTED_RATE ? `  ${labels.expectedRate}` : ""),
          ],
          [labels.noiseGate, Diagnostics.gate(d.noiseGate, labels)],
          [labels.capturing, Diagnostics.capture(d.capturing, labels)],
          [
            labels.sending,
            `${d.datagramsOut} audio datagrams/s` +
              (d.datagramsOut === 0 ? `  ${labels.nothingGoingOut}` : ""),
          ],
        ],
      },
      {
        title: labels.whatYouHear,
        rows: [
          [labels.device, d.outputDevice],
          [labels.sampleRate, rate(d.outputRate)],
          [labels.receiving, `${d.datagramsIn} datagrams/s`],
          [
            labels.mutedByYou,
            // The denominator is dropped when it is not known rather than printed as zero:
            // "1 of 0" is arithmetic nobody can act on, and the two figures come from different
            // places — the count from this client's own mixer, the population from the position
            // feed, which answers with nothing until it is connected.
            d.mutedOthers
              ? d.visiblePlayers > 0
                ? `${d.mutedOthers} of ${d.visiblePlayers}`
                : `${d.mutedOthers}`
              : labels.none,
          ],
        ],
      },
      {
        title: labels.link,
        rows: [
          [
            labels.state,
            d.reconnecting
              ? `${labels.reconnecting} (${d.attempt ?? 1})`
              : d.stalled
                ? `${labels.connected}  ${Diagnostics.duration(d.uptimeSeconds)}  ${labels.stalled}`
                : `${labels.connected}  ${Diagnostics.duration(d.uptimeSeconds)}`,
          ],
          [labels.roundTrip, `${d.rtt} ms`],
          [labels.packetLoss, `${d.lossPercent} %`],
          [labels.jitterBuffer, `${d.jitterMs} ms  /  ${d.jitterDrops} ${labels.drops}`],
          [labels.quicPort, `${d.quicPort}${d.quicPort !== 443 ? `  ${labels.fallbackPort}` : ""}`],
        ],
      },
      {
        title: labels.session,
        rows: [
          [labels.server, d.server],
          [labels.protocol, d.protocol],
          [labels.proximityRange, `${d.rangeMetres} m`],
          [labels.falloff, d.falloff],
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
