import { describe, expect, it } from "vitest";
import { DiagnosticsView } from "../../../js/app/dashboard/DiagnosticsView";
import type { VoiceDiagnostics } from "../../../js/app/dashboard/SelfController";
import type { MicActivity } from "../../../js/app/dashboard/PlayerLevelSources";
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

/** Defaulted for the same reason as `backend()`. */
function mic(over: Partial<MicActivity> = {}): MicActivity {
    return {
        attached: true,
        events: 120,
        failures: 0,
        eventsPerSecond: 10,
        lastRms: 0.041,
        silentForMs: 90,
        ...over,
    };
}

function voice(over: Partial<VoiceDiagnostics> = {}): VoiceDiagnostics {
    return {
        backend: backend(),
        mic: mic(),
        ...over,
    };
}

/**
 * `capturing` defaults to 0 — nothing coming off the device — so a case that says nothing
 * about the backend gets the reading that lets this row blame the capture stream.
 */
function row(
    v: VoiceDiagnostics | null,
    label: string,
    uiMode?: "activated" | "ptt",
    capturing: number | null = 0,
): string {
    const found = DiagnosticsView.voiceGroup(v, uiMode, capturing).rows.find(
        ([name]) => name === label,
    );
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
            mic: mic({ events: 300, lastRms: 0, silentForMs: 80 }),
        });
        expect(row(muted, "Capture stream")).toContain("10.0/s");
        expect(row(muted, "Capture stream")).not.toContain("not running");

        const dead = voice({ mic: mic({ events: 0, eventsPerSecond: 0, lastRms: 0, silentForMs: null }) });
        expect(row(dead, "Capture stream")).toContain("not running");
    });

    // A stream that emitted and then stopped is a third state again, and the rate alone
    // would keep reporting the average it built up before it died.
    it("calls out a stream that has stopped emitting", () => {
        const stalled = voice({
            mic: mic({ events: 300, lastRms: 0.02, silentForMs: 4200 }),
        });
        expect(row(stalled, "Capture stream")).toContain("stopped 4s ago");
    });

    it("says nothing about staleness while events are arriving", () => {
        expect(row(voice(), "Capture stream")).not.toContain("stopped");
    });

    /*
     * This row counts events arriving in this window, and reported their absence as "the
     * capture stream is not running" — a claim about the backend it has no way to make. A
     * phone carrying audio in both directions read as a dead microphone, and the report was
     * believed over the audio.
     */
    it("does not blame the capture stream while the backend is capturing", () => {
        const dead = voice({ mic: mic({ events: 0, eventsPerSecond: 0, lastRms: 0, silentForMs: null }) });
        const said = row(dead, "Capture stream", undefined, 50);
        expect(said).not.toContain("not running");
        expect(said).toContain("the microphone is fine");
        expect(said).toContain("50 frames/s");
    });

    it("still blames the capture stream when nothing is being captured either", () => {
        const dead = voice({ mic: mic({ events: 0, eventsPerSecond: 0, lastRms: 0, silentForMs: null }) });
        expect(row(dead, "Capture stream", undefined, 0)).toContain("not running");
    });

    /*
     * A listener that never registered and a capture stream that stopped both leave the count
     * at zero, and this accused the second for both. They are not fixed in the same place, so
     * a readout that cannot separate them sends the reader to the wrong half of the app.
     */
    /*
     * A handler that throws leaves the listener registered and the count where it was, which
     * reads identically to an event that never arrived — so the readout blames the transport
     * for a fault inside the window, and nothing anywhere contradicts it.
     */
    it("separates events it could not handle from events that never came", () => {
        const failing = voice({ mic: mic({ events: 40, failures: 40 }) });
        const said = row(failing, "Capture stream", undefined, 50);
        expect(said).toContain("could not be read");
        expect(said).not.toContain("no events");
    });

    it("reports a meter that never attached as its own fault", () => {
        const detached = voice({
            mic: mic({ attached: false, events: 0, eventsPerSecond: 0, lastRms: 0, silentForMs: null }),
        });
        const said = row(detached, "Capture stream", undefined, 50);
        expect(said).toContain("failed to register");
        expect(said).not.toContain("not running");
    });
});

/**
 * The row that separates "levels never reached the pill" from "the pill did not draw them".
 *
 * Both have now happened on the same phone, and from the outside they are the same flat
 * meter. The probe snapshot carries what the binding was handed and what it painted, and
 * this row turns the gap into a sentence.
 */
describe("the self meter row", () => {
    function meterRow(meter: Parameters<typeof DiagnosticsView.voiceGroup>[3]): string {
        const found = DiagnosticsView.voiceGroup(voice(), undefined, 50, meter).rows.find(
            ([name]) => name === "Self meter",
        );
        return found?.[1] ?? "";
    }

    it("is absent when no probe snapshot is supplied", () => {
        expect(meterRow(undefined)).toBe("");
    });

    it("says the pill is not mounted rather than inventing a state", () => {
        expect(
            meterRow({ mounted: false, levels: 0, lastLevel: 0, levelAgeMs: null, paints: 0, paintAgeMs: null }),
        ).toContain("not mounted");
    });

    it("reports a mounted meter that nothing has pushed to", () => {
        expect(
            meterRow({ mounted: true, levels: 0, lastLevel: 0, levelAgeMs: null, paints: 0, paintAgeMs: null }),
        ).toContain("no levels");
    });

    it("blames the renderer when levels arrived and nothing was ever painted", () => {
        const said = meterRow({
            mounted: true,
            levels: 40,
            lastLevel: 0.5,
            levelAgeMs: 300,
            paints: 0,
            paintAgeMs: null,
        });
        expect(said).toContain("none were painted");
        expect(said).toContain("renderer");
    });

    it("blames the renderer when painting stopped while levels keep arriving", () => {
        const said = meterRow({
            mounted: true,
            levels: 400,
            lastLevel: 0.5,
            levelAgeMs: 300,
            paints: 90,
            paintAgeMs: 9000,
        });
        expect(said).toContain("stopped painting");
    });

    it("reports a painting meter with its level", () => {
        const said = meterRow({
            mounted: true,
            levels: 400,
            lastLevel: 0.62,
            levelAgeMs: 300,
            paints: 900,
            paintAgeMs: 40,
        });
        expect(said).toContain("painting");
        expect(said).toContain("0.62");
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
            mic: mic({ events: 0, eventsPerSecond: 0, lastRms: 0, silentForMs: null }),
            error: "command not found",
        });
        expect(failed.rows[0]?.[1]).toContain("could not read");
        expect(failed.rows[0]?.[1]).toContain("command not found");
    });
});
