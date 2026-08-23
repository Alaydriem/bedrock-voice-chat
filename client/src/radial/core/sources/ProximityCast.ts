import { MarkData } from "../mark/MarkData";
import type { RingSource } from "../ring/RingSource";
import { PositionalSource } from "./PositionalSource";

export interface CastMember {
  name: string;
  hue: string;
  /** Radians, -PI/2 straight up. */
  bearing: number;
  /** Radians per millisecond: the player walking around you. */
  drift: number;
  /** Distance as a fraction of range, before breathing. */
  radius: number;
  /** How fast that distance breathes in and out. */
  breathe: number;
}

/**
 * The cast that demonstrates proximity voice: players placed around you, talking, at
 * distances that drift.
 *
 * One definition, because more than one screen shows it, and a second copy of a
 * demonstration is a second thing to keep true.
 *
 * Pure in `t`: no canvas, no timers, no state. The caller owns the loop.
 */
export class ProximityCast {
  /**
   * Distances are load-bearing rather than cosmetic. Falloff is quadratic, so a player at
   * 0.7 of range contributes (1 - 0.7)² = 0.09 — under the meter's own 0.08 speaking
   * threshold, which greys a row out. Anyone meant to look audible has to actually be near.
   */
  static readonly ROSTER: readonly CastMember[] = [
    { name: "ALAYDRIEM", hue: MarkData.hueAt(1), bearing: 0.55, drift: 0.0004, radius: 0.14, breathe: 0.00009 },
    { name: "PETRA", hue: MarkData.hueAt(4), bearing: 1.8, drift: -0.00029, radius: 0.22, breathe: 0.00006 },
    { name: "JUNO", hue: MarkData.hueAt(8), bearing: 3.0, drift: 0.00022, radius: 0.3, breathe: 0.00004 },
    { name: "MARROW", hue: MarkData.hueAt(11), bearing: 4.2, drift: -0.00018, radius: 0.38, breathe: 0.00007 },
    { name: "VESPER", hue: MarkData.hueAt(14), bearing: 5.4, drift: 0.00031, radius: 0.26, breathe: 0.00005 },
    { name: "CASS", hue: MarkData.hueAt(17), bearing: 2.4, drift: 0.00026, radius: 0.34, breathe: 0.00008 },
    { name: "RILEY", hue: MarkData.hueAt(19), bearing: 4.9, drift: -0.00024, radius: 0.2, breathe: 0.00005 },
    { name: "ODEN", hue: MarkData.hueAt(22), bearing: 1.1, drift: 0.00019, radius: 0.42, breathe: 0.00006 },
  ];

  /**
   * A voice that is always saying something.
   *
   * Floored so it never reaches zero: below 0.03 a voice leaves the ring entirely and its
   * row greys out, and a screen demonstrating constant activity cannot afford to look dead
   * half the time. `SyntheticLevelSource` gates half its cycle to exact silence, which is
   * honest for a roster and wrong here. Same two-sine shape as the mark's own dance.
   */
  static voice(t: number, index: number): number {
    const phase = (index * Math.PI * 2) / ProximityCast.ROSTER.length;
    const a = 0.5 + 0.5 * Math.sin(t * 0.0027 + phase);
    const b = 0.5 + 0.5 * Math.sin(t * 0.0011 + phase * 1.7);
    return 0.34 + 0.66 * (a * 0.62 + b * 0.38);
  }

  /** Metres from the listener, breathing in and out around the member's resting radius. */
  static distance(index: number, t: number): number {
    const member = ProximityCast.ROSTER[index];
    const fraction = Math.max(
      0.08,
      Math.min(0.55, member.radius + Math.sin(t * member.breathe + index) * 0.08),
    );
    return fraction * PositionalSource.RANGE;
  }

  /** One member's voice on the ring, or null once distance has taken it under the floor. */
  static placement(index: number, t: number): RingSource | null {
    const member = ProximityCast.ROSTER[index];
    return PositionalSource.toRingSource(
      {
        bearing: member.bearing + t * member.drift,
        distance: ProximityCast.distance(index, t),
        hue: member.hue,
      },
      ProximityCast.voice(t, index),
    );
  }

  /**
   * The first `count` members, placed. Out-of-range members are absent rather than silent,
   * so the length is also the number in earshot.
   */
  static placements(t: number, count: number): RingSource[] {
    const sources: RingSource[] = [];
    for (let i = 0; i < Math.min(count, ProximityCast.ROSTER.length); i++) {
      const source = ProximityCast.placement(i, t);
      if (source) sources.push(source);
    }
    return sources;
  }
}
