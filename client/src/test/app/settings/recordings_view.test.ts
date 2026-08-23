import { describe, expect, it } from "vitest";
import { RecordingsView } from "../../../js/app/settings/RecordingsView";
import type { RecordingSession } from "../../../js/bindings/RecordingSession";

function session(overrides: Record<string, unknown> = {}): RecordingSession {
    const { file_size_mb = 412, exportable = true, ...manifest } = overrides;
    return {
        file_size_mb,
        exportable,
        recording_path: "C:/recordings/01J8Z9",
        session_data: {
            session_id: "01J8Z9",
            start_timestamp: 1_753_732_440_000n,
            end_timestamp: null,
            duration_ms: 6_128_000n,
            emitter_player: "Alaydriem",
            participants: ["Alaydriem", "Petra", "Juno"],
            jukebox_participants: [],
            created_at: "1753732440",
            recording_version: "1",
            name: null,
            ...manifest,
        },
    } as unknown as RecordingSession;
}

describe("RecordingsView.duration", () => {
    it("shows hours only when there are hours", () => {
        expect(RecordingsView.duration(6_128_000)).toBe("1:42:08");
        expect(RecordingsView.duration(3_511_000)).toBe("58:31");
    });

    // A clip padded to 00:02:17 reads as a much longer recording at a glance.
    it("does not pad a short clip with an hours column", () => {
        expect(RecordingsView.duration(137_000)).toBe("2:17");
    });

    // A session still being written has no duration yet, and 0:00 is a claim that it is
    // empty rather than that it is ongoing.
    it("says nothing for a recording that has not finished", () => {
        expect(RecordingsView.duration(null)).toBe("—");
        expect(RecordingsView.duration(0)).toBe("—");
    });
});

describe("RecordingsView.size", () => {
    it("stays in megabytes below a gigabyte", () => {
        expect(RecordingsView.size(412 * 1024 * 1024)).toBe("412 MB");
    });

    // "0.4 GB" reads as nothing at all next to a list of real sizes.
    it("switches to gigabytes only once there is one", () => {
        expect(RecordingsView.size(1.6 * 1024 * 1024 * 1024)).toBe("1.6 GB");
    });

    // A short session weighs well under a megabyte, and "0 MB" reads as a broken recording
    // rather than a small one.
    it("drops to kilobytes below a megabyte", () => {
        expect(RecordingsView.size(340 * 1024)).toBe("340 KB");
    });

    it("drops to bytes below a kilobyte", () => {
        expect(RecordingsView.size(512)).toBe("512 B");
    });

    it("says nothing was written for an empty session", () => {
        expect(RecordingsView.size(0)).toBe("0 KB");
    });
});

describe("RecordingsView.row", () => {
    // The session id is unique and unmemorable, so an unnamed recording is identified by
    // when it happened. The manifest holds unix seconds, which no reader can parse.
    it("falls back to the recorded time when the session has no name", () => {
        const row = RecordingsView.row(session());
        expect(row.unnamed).toBe(true);
        expect(row.name).toBe(row.recorded);
        expect(row.name).not.toContain("1753732440");
    });

    it("prefers the name once one is set", () => {
        const row = RecordingsView.row(session({ name: "Nether run" }));
        expect(row.name).toBe("Nether run");
        expect(row.unnamed).toBe(false);
    });

    // Whitespace is not a name. Accepting one produces a row that looks deleted.
    it("treats a blank name as no name", () => {
        expect(RecordingsView.row(session({ name: "   " })).unnamed).toBe(true);
    });

    // The participant list is who was heard, which is not the same as what can be
    // exported: it never named you, and it never named the jukebox.
    it("counts the people the session heard", () => {
        expect(RecordingsView.row(session()).players).toBe(3);
    });

    // Sorting on the label would order 1 February above 9 January.
    it("carries a sortable timestamp beside the label", () => {
        expect(RecordingsView.row(session()).recordedAt).toBe(1_753_732_440_000);
    });

    it("carries the exportable flag through", () => {
        expect(RecordingsView.row(session({ exportable: false })).exportable).toBe(false);
    });
});

describe("RecordingsView.totalSize", () => {
    it("adds the sessions up rather than the rounded labels", () => {
        const rows = RecordingsView.rows([session({ file_size_mb: 700 }), session({ file_size_mb: 700 })]);
        expect(RecordingsView.totalSize(rows)).toBe("1.4 GB");
    });
});
