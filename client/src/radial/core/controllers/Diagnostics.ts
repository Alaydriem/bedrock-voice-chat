export type Severity = "ok" | "warn" | "bad";

export interface DiagnosticsInput {
  /** Round-trip time in milliseconds. */
  rtt: number;
  lossPercent: number;
  jitterMs: number;
  jitterDrops: number;
  datagramsIn: number;
  datagramsOut: number;
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
  muted: boolean;
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
    if (d.deafened) return ["warn", "You are deafened. You cannot hear anyone."];
    if (d.muted) return ["bad", "You are muted. Nobody can hear you."];
    if (d.pttIdle) return ["warn", "Push-to-talk is on. Hold the mic button to speak."];
    if (d.inputRate !== Diagnostics.EXPECTED_RATE) {
      return [
        "warn",
        `Your input device is running at ${(d.inputRate / 1000).toFixed(1)} kHz. BVC expects 48 kHz.`,
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
          ["Noise gate", d.muted ? "muted" : "open"],
          [
            "Sending",
            `${d.datagramsOut} datagrams/s` + (d.datagramsOut === 0 ? "  ← nothing is going out" : ""),
          ],
        ],
      },
      {
        title: "What you hear",
        rows: [
          ["Device", d.outputDevice],
          ["Sample rate", rate(d.outputRate)],
          ["Receiving", `${d.datagramsIn} datagrams/s`],
          ["Muted by you", d.mutedOthers ? `${d.mutedOthers} of ${d.visiblePlayers}` : "none"],
        ],
      },
      {
        title: "Link",
        rows: [
          [
            "State",
            d.reconnecting
              ? `reconnecting (${d.attempt ?? 1})`
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
