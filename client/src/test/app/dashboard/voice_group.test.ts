import { describe, expect, it } from "vitest";
import { DiagnosticsView } from "../../../js/app/dashboard/DiagnosticsView";
import type { VoiceDiagnostics } from "../../../js/app/dashboard/SelfController";
import type { VoiceRuntimeState } from "../../../js/bindings/VoiceRuntimeState";

/**
 * What the backend reports, defaulted so a case states only what it cares about.
 *
 * Each case used to write the DTO out whole, so adding one field to it broke every case
 * here at once — none of which had anything to say about that field.
 */
function backend(over: Partial<VoiceRuntimeState> = {}): VoiceRuntimeState {
    return {
        voiceMode: "openMic",
        pttActive: false,
        inputMuted: false,
        outputMuted: false,
        recording: false,
        ...over,
    };
}

function voice(over: Partial<VoiceDiagnostics> = {}): VoiceDiagnostics {
    return {
        backend: backend(),
        mic: { events: 120, eventsPerSecond: 10, lastRms: 0.041, silentForMs: 90 },
        ...over,
    };
}

function row(v: VoiceDiagnostics | null, label: string, uiMode?: "activated" | "ptt"): string {
    const found = DiagnosticsView.voiceGroup(v, uiMode).rows.find(([name]) => name === label);
    return found?.[1] ?? "";
}

describe("the voice group", () => {
    it("reports the mode the backend holds", () => {
        expect(row(voice(), "Mode")).toBe("open mic");
        expect(
            row(voice({ backend: backend({ voiceMode: "pushToTalk", pttActive: false, inputMuted: true, outputMuted: false  }) }), "Mode"),
        ).toBe("push-to-talk");
    });

    // The one question the dashboard cannot answer: the mic button never draws the muted
    // glyph in push-to-talk, because muted is that mode's resting state.
    it("says whether the microphone is open", () => {
        expect(row(voice(), "Microphone")).toBe("open");
        expect(
            row(voice({ backend: backend({ voiceMode: "openMic", pttActive: false, inputMuted: true, outputMuted: false  }) }), "Microphone"),
        ).toContain("muted");
    });

    // Muted in open mic is a fault; muted in push-to-talk is the resting state. The same
    // flag means opposite things, so the row says which one this is.
    it("distinguishes a resting mute from a broken one", () => {
        const resting = row(
            voice({ backend: backend({ voiceMode: "pushToTalk", pttActive: false, inputMuted: true, outputMuted: false  }) }),
            "Microphone",
        );
        expect(resting).toContain("resting state");

        const broken = row(
            voice({ backend: backend({ voiceMode: "openMic", pttActive: false, inputMuted: true, outputMuted: false  }) }),
            "Microphone",
        );
        expect(broken).toContain("nothing is being captured");
    });

    it("shows whether a hold is registered", () => {
        const held = voice({
            backend: backend({ voiceMode: "pushToTalk", pttActive: true, inputMuted: false, outputMuted: false  }),
        });
        expect(row(held, "Hold")).toBe("held");

        const released = voice({
            backend: backend({ voiceMode: "pushToTalk", pttActive: false, inputMuted: true, outputMuted: false  }),
        });
        expect(row(released, "Hold")).toBe("released");
    });

    // A hold state in open mic would invite reading it as a fault when it is simply not
    // that mode's control.
    it("does not report a hold in open mic", () => {
        expect(row(voice(), "Hold")).toContain("n/a");
    });
});

describe("the mode the button believes", () => {
    const backendPtt = backend({ voiceMode: "pushToTalk" as const, pttActive: false, inputMuted: true, outputMuted: false });

    /**
     * The mode reaches the button by event, and the button is what turns a tap into a hold.
     * A lost event leaves the two disagreeing silently: the backend refuses holds it never
     * heard asked for, and the button keeps offering the toggle it last knew about.
     */
    it("calls out a button that never heard the mode change", () => {
        const shown = row(voice({ backend: backendPtt }), "Mode", "activated");
        expect(shown).toContain("push-to-talk");
        expect(shown).toContain("the button still thinks open mic");
    });

    it("says nothing extra when the two agree", () => {
        expect(row(voice({ backend: backendPtt }), "Mode", "ptt")).toBe("push-to-talk");
        expect(row(voice(), "Mode", "activated")).toBe("open mic");
    });

    // The reverse is a real state too: settings reverted, the backend did not.
    it("calls out the disagreement in the other direction", () => {
        expect(row(voice(), "Mode", "ptt")).toContain("the button still thinks push-to-talk");
    });
});

describe("the capture stream row", () => {
    /**
     * The distinction the whole group exists for.
     *
     * A muted input emits at rms 0 and a dead stream emits nothing, and the meter draws
     * both as a flat line. Only the event count tells them apart.
     */
    it("separates a muted stream from one that is not running", () => {
        const muted = voice({
            backend: backend({ voiceMode: "pushToTalk", pttActive: false, inputMuted: true, outputMuted: false  }),
            mic: { events: 300, eventsPerSecond: 10, lastRms: 0, silentForMs: 80 },
        });
        expect(row(muted, "Capture stream")).toContain("10.0/s");
        expect(row(muted, "Capture stream")).not.toContain("not running");

        const dead = voice({ mic: { events: 0, eventsPerSecond: 0, lastRms: 0, silentForMs: null } });
        expect(row(dead, "Capture stream")).toContain("not running");
    });

    // A stream that emitted and then stopped is a third state again, and the rate alone
    // would keep reporting the average it built up before it died.
    it("calls out a stream that has stopped emitting", () => {
        const stalled = voice({
            mic: { events: 300, eventsPerSecond: 10, lastRms: 0.02, silentForMs: 4200 },
        });
        expect(row(stalled, "Capture stream")).toContain("stopped 4s ago");
    });

    it("says nothing about staleness while events are arriving", () => {
        expect(row(voice(), "Capture stream")).not.toContain("stopped");
    });
});

describe("the voice group before it can report", () => {
    it("says so rather than drawing a state it does not have", () => {
        expect(row(null, "State")).toBe("not read yet");
    });

    // A failed probe is not "open mic, unmuted" — that is a state, and reporting it would
    // be the exact lie this group exists to stop.
    it("reports a probe that failed, with the reason", () => {
        const failed = DiagnosticsView.voiceGroup({
            backend: null,
            mic: { events: 0, eventsPerSecond: 0, lastRms: 0, silentForMs: null },
            error: "command not found",
        });
        expect(failed.rows[0]?.[1]).toContain("could not read");
        expect(failed.rows[0]?.[1]).toContain("command not found");
    });
});
