import { describe, expect, it } from "vitest";
import { RecordingTracksView } from "../../../js/app/settings/RecordingTracksView";
import type { RecordingTrack } from "../../../js/bindings/RecordingTrack";

const own: RecordingTrack = { keys: ["minecraft:Alaydriem"], display: "Alaydriem", kind: "Own" };
const petra: RecordingTrack = { keys: ["minecraft:Petra"], display: "Petra", kind: "Player" };
const juno: RecordingTrack = { keys: ["minecraft:Juno"], display: "Juno", kind: "Player" };
const jukebox: RecordingTrack = {
    keys: ["jukebox:rain", "jukebox:sting"],
    display: "Jukebox",
    kind: "Jukebox",
};

describe("RecordingTracksView.groups", () => {
    it("puts you first, without a heading over you", () => {
        const groups = RecordingTracksView.groups([own, petra]);

        expect(groups[0].heading).toBeNull();
        expect(groups[0].tracks).toEqual([own]);
    });

    it("heads the players group only when there are players", () => {
        const groups = RecordingTracksView.groups([own, petra, juno]);

        expect(groups[1].heading).toBe("players");
        expect(groups[1].tracks).toEqual([petra, juno]);
    });

    it("omits a group with nothing in it rather than heading an empty list", () => {
        const groups = RecordingTracksView.groups([own]);

        expect(groups).toHaveLength(1);
        expect(groups.some((g) => g.heading === "players")).toBe(false);
    });

    it("keeps the jukebox as one row at the end", () => {
        const groups = RecordingTracksView.groups([own, petra, jukebox]);

        expect(groups[groups.length - 1].tracks).toEqual([jukebox]);
    });

    it("returns nothing for a session with no tracks", () => {
        expect(RecordingTracksView.groups([])).toEqual([]);
    });
});

describe("RecordingTracksView.keysFor", () => {
    it("expands a track into every key behind it", () => {
        const keys = RecordingTracksView.keysFor([own, jukebox], new Set(["Jukebox"]));

        expect(keys).toEqual(["jukebox:rain", "jukebox:sting"]);
    });

    it("ignores a name that is not a track in this session", () => {
        expect(RecordingTracksView.keysFor([own], new Set(["Ghost"]))).toEqual([]);
    });
});

describe("RecordingTracksView.sourceNote", () => {
    it("counts the sources behind a jukebox track", () => {
        expect(RecordingTracksView.sourceNote(jukebox)).toBe("2 sources");
    });

    it("says nothing about a voice, which is always one thing", () => {
        expect(RecordingTracksView.sourceNote(petra)).toBe("");
    });
});

describe("RecordingTracksView.summary", () => {
    it("reports a clean run by what it wrote", () => {
        const text = RecordingTracksView.summary({ written: ["Alaydriem", "Petra"], failed: [] });

        expect(text).toBe("2 tracks written");
    });

    // Silence here reads as a complete export, which is the defect this replaces.
    it("names the tracks that failed", () => {
        const text = RecordingTracksView.summary({
            written: ["Alaydriem"],
            failed: [{ track: "Petra", reason: "no such file" }],
        });

        expect(text).toBe("1 of 2 written — Petra failed");
    });

    it("does not pluralise a single track", () => {
        expect(RecordingTracksView.summary({ written: ["Petra"], failed: [] })).toBe(
            "1 track written",
        );
    });
});
