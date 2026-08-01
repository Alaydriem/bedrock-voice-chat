export type VoiceMode = "activated" | "ptt";

export interface SelfSnapshot {
  muted: boolean;
  deafened: boolean;
  recording: boolean;
  mode: VoiceMode;
  /** True while push-to-talk is held. Meaningless in `activated` mode. */
  holding: boolean;
  /** True when audio is actually leaving this machine. */
  transmitting: boolean;
}

/**
 * Mute, deafen, record, push-to-talk.
 *
 * Three invariants, each of them a bug the first time it is missed:
 *
 *  - **Deafen implies mute.** Hearing nobody while they still hear you is a state
 *    people end up in by accident and cannot detect from their own screen.
 *  - **Unmute undeafens.** Someone who presses the mic button wants to be in the
 *    conversation. Leaving them deafened means the fix for "I can't hear anyone" is a
 *    second button they have no reason to suspect.
 *  - **Push-to-talk is a mode of the mic button, not a sibling control.** In PTT the
 *    mic button becomes a hold control; there is no separate mute, because not holding
 *    it already is mute.
 *
 * Emits a snapshot rather than exposing setters, so every surface showing self state —
 * the desktop pill, the phone capsule, the frame's edge stripe, the status verdict —
 * is a function of one value and they cannot disagree.
 */
export class SelfState {
  #muted = false;
  #deafened = false;
  #recording = false;
  #mode: VoiceMode = "activated";
  #holding = false;
  #recordStartedAt = 0;
  #listeners = new Set<(s: SelfSnapshot) => void>();

  get snapshot(): SelfSnapshot {
    return {
      muted: this.#muted,
      deafened: this.#deafened,
      recording: this.#recording,
      mode: this.#mode,
      holding: this.#holding,
      transmitting: this.transmitting,
    };
  }

  /** Whether audio is leaving this machine right now. */
  get transmitting(): boolean {
    if (this.#mode === "ptt") return this.#holding;
    return !this.#muted;
  }

  /** Milliseconds since recording was armed, or 0. */
  elapsed(now: number): number {
    return this.#recording ? Math.max(0, now - this.#recordStartedAt) : 0;
  }

  subscribe(listener: (s: SelfSnapshot) => void): () => void {
    this.#listeners.add(listener);
    listener(this.snapshot);
    return () => this.#listeners.delete(listener);
  }

  toggleMute(): void {
    // Deafened is the louder state, so the mic button clears it first: one press
    // gets you all the way back into the conversation.
    if (this.#deafened) {
      this.#deafened = false;
      this.#muted = false;
    } else {
      this.#muted = !this.#muted;
    }
    this.#emit();
  }

  toggleDeafen(): void {
    this.#deafened = !this.#deafened;
    this.#muted = this.#deafened;
    this.#emit();
  }

  setMode(mode: VoiceMode): void {
    this.#mode = mode;
    // Leaving mute set on entering PTT would make the hold control silently do
    // nothing, which reads as a broken button rather than as a muted mic.
    this.#muted = false;
    this.#holding = false;
    this.#emit();
  }

  /** Push-to-talk key or button held. Ignored outside PTT mode. */
  hold(down: boolean): void {
    if (this.#mode !== "ptt") return;
    if (this.#holding === down) return;
    this.#holding = down;
    this.#emit();
  }

  toggleRecording(now: number): void {
    this.#recording = !this.#recording;
    if (this.#recording) this.#recordStartedAt = now;
    this.#emit();
  }

  reset(): void {
    this.#muted = false;
    this.#deafened = false;
    this.#recording = false;
    this.#mode = "activated";
    this.#holding = false;
    this.#emit();
  }

  #emit(): void {
    const snapshot = this.snapshot;
    for (const listener of this.#listeners) listener(snapshot);
  }
}
