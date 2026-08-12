import { I18n } from "$lib/i18n";
import type { DiagnosticsLabels, Verdict } from "$radial/core/controllers/Diagnostics";

/**
 * What a verdict reads as.
 *
 * The kit decides which check failed and hands back a code; this decides the words. Keeping
 * the two apart is what lets the kit stay framework-free — it runs under `node --test` with
 * no bundler and no Svelte, so it can neither import the translation surface nor evaluate
 * the runes it is built on.
 */
export default class DiagnosticsCopy {
  /**
   * The panel's labels, translated.
   *
   * Read on each call rather than built once, so a language change re-renders the panel —
   * the caller reads this inside a derivation and the runes track it from there.
   */
  static labels(): DiagnosticsLabels {
    return {
      yourMic: I18n.t("Your mic"),
      whatYouHear: I18n.t("What you hear"),
      link: I18n.t("Link"),
      session: I18n.t("Session"),

      device: I18n.t("Device"),
      sampleRate: I18n.t("Sample rate"),
      noiseGate: I18n.t("Noise gate"),
      capturing: I18n.t("Capturing"),
      sending: I18n.t("Sending"),
      receiving: I18n.t("Receiving"),
      mutedByYou: I18n.t("Muted by you"),
      state: I18n.t("State"),
      roundTrip: I18n.t("Round trip"),
      packetLoss: I18n.t("Packet loss"),
      jitterBuffer: I18n.t("Jitter buffer"),
      quicPort: I18n.t("QUIC port"),
      wssPort: I18n.t("WSS port"),
      connectionType: I18n.t("Connection type"),
      server: I18n.t("Server"),
      protocol: I18n.t("Protocol"),
      proximityRange: I18n.t("Proximity range"),
      falloff: I18n.t("Falloff"),

      gateOff: I18n.t("off (not in the audio path)"),
      gateOpen: I18n.t("on, open (passing audio)"),
      gateClosed: I18n.t("on, closed  ← this is cutting your mic"),
      notMeasuredYet: I18n.t("not measured yet"),
      micStopped: I18n.t("← your microphone has stopped"),
      nothingGoingOut: I18n.t("← nothing is going out"),
      expectedRate: I18n.t("← expected 48.0"),
      none: I18n.t("none"),
      reconnecting: I18n.t("reconnecting"),
      connected: I18n.t("connected"),
      stalled: I18n.t("← stalled"),
      fallbackPort: I18n.t("(fallback)"),
      drops: I18n.t("drops"),
    };
  }

  static of(verdict: Verdict): string {
    const params = verdict.params ?? {};

    switch (verdict.code) {
      case "reconnecting":
        // Two sentences rather than one with an optional clause: a translator cannot move
        // "— attempt 3" to where their language wants it if it arrives pre-joined.
        return Number(params.attempt) > 0
          ? I18n.tf("Reconnecting — attempt {attempt}. Nobody can hear you right now.", params)
          : I18n.t("Reconnecting. Nobody can hear you right now.");

      case "stalled":
        return I18n.t("Your audio is not reaching the server. Try reconnecting.");

      case "deafened":
        return I18n.t("You are deafened. You cannot hear anyone.");

      case "ptt-idle":
        return I18n.t("Push-to-talk is on. Hold the mic button to speak.");

      case "muted":
        return I18n.t("You are muted. Nobody can hear you.");

      case "input-rate":
        return I18n.tf(
          "Your input device is running at {kHz} kHz. BVC expects 48 kHz.",
          params,
        );

      case "concealment":
        return I18n.tf(
          "{percent}% of the worst speaker's audio had to be reconstructed. They will sound rough.",
          params,
        );

      case "loss":
        return I18n.tf("Packet loss is {percent}%. Audio will break up.", params);

      case "muted-others":
        // The one verdict with a count in it. `tn` picks the form the reader's language
        // needs — two in English, four in Polish — instead of the two English has.
        return I18n.tf(
          I18n.tn(
            "{count} player is muted by you.",
            "{count} players are muted by you.",
            Number(params.count),
          ),
          params,
        );

      case "fine":
        return I18n.t("Everything looks fine.");
    }
  }
}
