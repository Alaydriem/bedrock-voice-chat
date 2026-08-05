import { describe, expect, it } from "vitest";
import { SoundLibraryView } from "../../../js/app/settings/SoundLibraryView";
import type { AudioFileResponse } from "../../../js/bindings/AudioFileResponse";

function file(overrides: Record<string, unknown> = {}): AudioFileResponse {
    return {
        id: "snd_airhorn",
        original_filename: "airhorn.ogg",
        uploader: { gamertag: "Petra" },
        duration_ms: 2_000,
        file_size_bytes: 31_744,
        game: "minecraft",
        created_at: 1_753_732_440,
        ...overrides,
    } as unknown as AudioFileResponse;
}

describe("SoundLibraryView.row", () => {
    it("keeps the id, because that is what the jukebox command takes", () => {
        expect(SoundLibraryView.row(file()).id).toBe("snd_airhorn");
    });

    it("names the uploader", () => {
        expect(SoundLibraryView.row(file()).uploader).toBe("Petra");
    });

    // An empty cell reads as a rendering fault rather than as a fact about the row.
    it("names an uploader it cannot resolve", () => {
        expect(SoundLibraryView.row(file({ uploader: {} })).uploader).toBe("Unknown");
        expect(SoundLibraryView.row(file({ uploader: { gamertag: "  " } })).uploader).toBe("Unknown");
    });
});

describe("SoundLibraryView.size", () => {
    // A library is mostly two-second sound effects. Reporting those in megabytes makes
    // every row read as 0.0.
    it("stays in kilobytes for a sound effect", () => {
        expect(SoundLibraryView.size(31_744)).toBe("31 KB");
    });

    it("switches to megabytes for music", () => {
        expect(SoundLibraryView.size(5.2 * 1024 * 1024)).toBe("5.2 MB");
    });
});

describe("SoundLibraryView permissions", () => {
    // A delete button shown to somebody the server will refuse is a button that fails
    // after they press it.
    it("lets only a deleter manage rows", () => {
        expect(SoundLibraryView.canManage(["audio_upload"])).toBe(false);
        expect(SoundLibraryView.canManage(["audio_delete"])).toBe(true);
        expect(SoundLibraryView.canManage([])).toBe(false);
    });

    it("keeps upload and delete apart", () => {
        expect(SoundLibraryView.canUpload(["audio_upload"])).toBe(true);
        expect(SoundLibraryView.canDelete(["audio_upload"])).toBe(false);
    });
});

describe("SoundLibraryView.pageCount", () => {
    it("counts the pages a total needs", () => {
        expect(SoundLibraryView.pageCount(12, 5)).toBe(3);
        expect(SoundLibraryView.pageCount(10, 5)).toBe(2);
    });

    // An empty library is one empty page, not zero pages: a pager rendered over zero
    // divides by nothing and shows nothing to return to.
    it("is never zero", () => {
        expect(SoundLibraryView.pageCount(0, 5)).toBe(1);
    });
});

describe("SoundLibraryView page base", () => {
    // The server reads `page` as an index, so page 1 of a two-item library returns nothing.
    // A pager starting at 1 therefore reported "2 Sounds" above an empty table.
    it("starts at zero, as the server counts", () => {
        expect(SoundLibraryView.FIRST_PAGE).toBe(0);
    });

    it("holds a page inside the library", () => {
        expect(SoundLibraryView.clampPage(4, 2, 20)).toBe(0);
        expect(SoundLibraryView.clampPage(-1, 60, 20)).toBe(0);
        expect(SoundLibraryView.clampPage(2, 60, 20)).toBe(2);
        expect(SoundLibraryView.clampPage(9, 60, 20)).toBe(2);
    });

    it("keeps an empty library on its one page", () => {
        expect(SoundLibraryView.clampPage(1, 0, 20)).toBe(0);
    });
});

describe("SoundLibraryView.fileNameFrom", () => {
    it("takes the basename from a desktop path", () => {
        expect(SoundLibraryView.fileNameFrom(String.raw`C:\Users\al\airhorn.ogg`)).toBe(
            "airhorn.ogg",
        );
        expect(SoundLibraryView.fileNameFrom("/home/al/sounds/airhorn.ogg")).toBe("airhorn.ogg");
    });

    // Android's picker returns a content URI, not a path. Uploading its raw last segment
    // would name the sound `audio%3A1000000123`.
    it("does not use a content URI segment as a name", () => {
        const uri = "content://com.android.providers.media.documents/document/audio%3A1000000123";
        expect(SoundLibraryView.fileNameFrom(uri)).toBe("upload.ogg");
    });

    it("keeps a real name out of a content URI when there is one", () => {
        const uri = "content://com.android.externalstorage.documents/document/primary%3Abell.ogg";
        expect(SoundLibraryView.fileNameFrom(uri)).toBe("bell.ogg");
    });

    it("falls back for anything without an extension", () => {
        expect(SoundLibraryView.fileNameFrom("")).toBe("upload.ogg");
        expect(SoundLibraryView.fileNameFrom("/tmp/noextension")).toBe("upload.ogg");
    });
});
