import { render, waitFor } from "@testing-library/svelte";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { invokeCalls, mockInvoke } from "../tauri";

vi.mock("@tauri-apps/plugin-store", () => ({
    Store: { load: async () => ({ get: async () => "https://bvc.example.com", set: async () => {}, save: async () => {} }) },
}));

const { default: RecordingsPane } = await import(
    "../../components/settings/panes/RecordingsPane.svelte"
);
const { default: LibraryPane } = await import("../../components/settings/panes/LibraryPane.svelte");
const { default: WebSocketPane } = await import(
    "../../components/settings/panes/WebSocketPane.svelte"
);

function mount(component: unknown, props: Record<string, unknown> = {}) {
    const host = document.createElement("div");
    document.body.append(host);
    render(component as never, { target: host, props } as never);
    return {
        host,
        text: () => host.textContent ?? "",
        button: (label: string) =>
            host.querySelector<HTMLButtonElement>(`[aria-label^="${label}"]`),
    };
}

function session(overrides: Record<string, unknown> = {}) {
    const { file_size_mb = 412, exportable = true, ...manifest } = overrides;
    return {
        file_size_mb,
        exportable,
        recording_path: "C:/recordings/01J8Z9",
        session_data: {
            session_id: "01J8Z9",
            start_timestamp: 1_753_732_440_000,
            end_timestamp: null,
            duration_ms: 6_128_000,
            emitter_player: "Alaydriem",
            participants: ["Alaydriem", "Petra"],
            jukebox_participants: [],
            created_at: "1753732440",
            recording_version: "1",
            name: null,
            ...manifest,
        },
    };
}

function sound(overrides: Record<string, unknown> = {}) {
    return {
        id: "snd_airhorn",
        original_filename: "airhorn.ogg",
        uploader: { gamertag: "Petra" },
        duration_ms: 2000,
        file_size_bytes: 31_744,
        game: "minecraft",
        created_at: 1_753_732_440,
        ...overrides,
    };
}

beforeEach(() => {
    mockInvoke({});
});

describe("RecordingsPane", () => {
    it("identifies an unnamed session by when it happened, never by its id", async () => {
        mockInvoke({ get_recording_sessions: () => [session()] });
        const view = mount(RecordingsPane);
        await waitFor(() => expect(view.text()).toContain("Recorded"));
        expect(view.text()).not.toContain("01J8Z9");
    });

    it("prefers the name once one is set", async () => {
        mockInvoke({ get_recording_sessions: () => [session({ name: "Nether run" })] });
        const view = mount(RecordingsPane);
        await waitFor(() => expect(view.text()).toContain("Nether run"));
    });

    const track = (display: string, kind: string, keys: string[] = [`minecraft:${display}`]) => ({
        keys,
        display,
        kind,
    });

    /**
     * Open the first session and wait for its tracks. The screen renders before the track
     * list arrives, so waiting on the button alone races the fetch behind it.
     */
    async function openFirst(view: ReturnType<typeof mount>) {
        await waitFor(() => expect(view.host.querySelector("tbody tr")).not.toBeNull());
        view.host.querySelector<HTMLElement>("tbody tr")?.click();
        await waitFor(() =>
            expect(view.host.querySelector(".rad-tracklist .rad-checkbox")).not.toBeNull(),
        );
    }

    const go = (view: ReturnType<typeof mount>) =>
        view.host.querySelector<HTMLButtonElement>("[data-export-go]");

    it("opens a session from its row and lists the tracks that session can write", async () => {
        mockInvoke({
            get_recording_sessions: () => [session()],
            get_recording_tracks: () => [track("Alaydriem", "Own"), track("Petra", "Player")],
        });
        const view = mount(RecordingsPane);
        await openFirst(view);

        expect(view.text()).toContain("Petra");
        expect(go(view)?.textContent).toContain("Export 2 tracks");
    });

    // The regression this replaces: your own voice was recorded and could never be picked.
    it("offers your own voice as a track", async () => {
        mockInvoke({
            get_recording_sessions: () => [session()],
            get_recording_tracks: () => [track("Alaydriem", "Own")],
        });
        const view = mount(RecordingsPane);
        await openFirst(view);

        expect(view.host.querySelector('[aria-label="Alaydriem"]')).not.toBeNull();
    });

    it("refuses to export when nothing is ticked", async () => {
        mockInvoke({
            get_recording_sessions: () => [session()],
            get_recording_tracks: () => [track("Petra", "Player")],
        });
        const view = mount(RecordingsPane);
        await openFirst(view);

        view.host.querySelector<HTMLElement>("[data-track-none]")?.click();

        await waitFor(() => expect(go(view)?.disabled).toBe(true));
    });

    it("sends the keys behind a track, not the name on the checkbox", async () => {
        mockInvoke({
            get_recording_sessions: () => [session()],
            get_recording_tracks: () => [
                track("Jukebox", "Jukebox", ["jukebox:rain", "jukebox:sting"]),
            ],
            export_recording: () => ({ written: ["Jukebox"], failed: [] }),
        });
        const view = mount(RecordingsPane);
        await openFirst(view);

        go(view)?.click();

        await waitFor(() => {
            const call = invokeCalls().find((c) => c.cmd === "export_recording");
            expect(call?.args).toMatchObject({
                tracks: [{ keys: ["jukebox:rain", "jukebox:sting"] }],
            });
        });
    });

    it("names the tracks that failed instead of reporting a clean export", async () => {
        mockInvoke({
            get_recording_sessions: () => [session()],
            get_recording_tracks: () => [track("Alaydriem", "Own"), track("Petra", "Player")],
            export_recording: () => ({
                written: ["Alaydriem"],
                failed: [{ track: "Petra", reason: "no such file" }],
            }),
        });
        const view = mount(RecordingsPane);
        await openFirst(view);

        go(view)?.click();

        await waitFor(() => expect(view.text()).toContain("Petra failed"));
    });

    // Still being written, or written by a build whose format this one cannot read.
    // Either way it can be named and deleted, but exporting it would produce nothing.
    it("says why a session cannot be exported rather than only disabling the button", async () => {
        mockInvoke({
            get_recording_sessions: () => [session({ exportable: false })],
            get_recording_tracks: () => [track("Alaydriem", "Own")],
        });
        const view = mount(RecordingsPane);
        await openFirst(view);

        expect(view.text()).toContain("This recording cannot be exported");
        expect(go(view)?.disabled).toBe(true);
    });

    it("goes back to the table without losing the list", async () => {
        mockInvoke({
            get_recording_sessions: () => [session({ name: "Nether run" })],
            get_recording_tracks: () => [track("Alaydriem", "Own")],
        });
        const view = mount(RecordingsPane);
        await openFirst(view);

        view.host.querySelector<HTMLElement>("[data-rec-back]")?.click();

        await waitFor(() => expect(view.text()).toContain("Nether run"));
        expect(view.host.querySelector("tbody tr")).not.toBeNull();
    });

    // Empty is a different screen from a folder that could not be read.
    it("says how a recording gets here when there are none", async () => {
        mockInvoke({ get_recording_sessions: () => [] });
        const view = mount(RecordingsPane);
        await waitFor(() => expect(view.text()).toContain("Nothing recorded yet"));
    });

    it("gives a reason and a retry when the folder cannot be read", async () => {
        mockInvoke({
            get_recording_sessions: () => {
                throw new Error("Failed to read recordings directory");
            },
        });
        const view = mount(RecordingsPane);
        await waitFor(() => expect(view.text()).toContain("Couldn't read your recordings"));
        expect(view.text()).toContain("Failed to read recordings directory");
    });

    function policy(recordingAllowed: boolean) {
        return () => ({
            voiceMode: "openMic",
            pttActive: false,
            inputMuted: false,
            outputMuted: false,
            recording: false,
            jukeboxPlaying: false,
            recordingAllowed,
        });
    }

    it("explains why recording is unavailable on a server that disallows it", async () => {
        mockInvoke({
            get_recording_sessions: () => [session()],
            voice_runtime_state: policy(false),
        });
        const view = mount(RecordingsPane);

        await waitFor(() =>
            expect(view.text()).toContain("This server does not allow recording"),
        );
    });

    // Turning recording off must not read as losing what was already recorded.
    it("still lists and offers to export existing sessions when recording is disallowed", async () => {
        mockInvoke({
            get_recording_sessions: () => [session({ name: "Nether run" })],
            get_recording_tracks: () => [track("Alaydriem", "Own")],
            voice_runtime_state: policy(false),
        });
        const view = mount(RecordingsPane);

        await waitFor(() => expect(view.text()).toContain("Nether run"));
        await openFirst(view);
        expect(go(view)?.disabled).toBe(false);
    });

    it("shows no notice where the server allows recording", async () => {
        mockInvoke({
            get_recording_sessions: () => [session()],
            voice_runtime_state: policy(true),
        });
        const view = mount(RecordingsPane);

        await waitFor(() => expect(view.text()).toContain("Recorded"));
        expect(view.host.querySelector(".rad-callout--warn")).toBeNull();
    });
});

describe("LibraryPane", () => {
    function library(permissions: string[], items = [sound()], total = items.length) {
        mockInvoke({
            get_credential: () => JSON.stringify({ allowed: permissions }),
            list_audio_files: () => ({ items, total, page: 1, page_size: 8 }),
        });
    }

    it("shows the sound id, because that is what the jukebox command takes", async () => {
        library(["audio_upload", "audio_delete"]);
        const view = mount(LibraryPane);
        await waitFor(() => expect(view.text()).toContain("snd_airhorn"));
    });

    // A delete button shown to somebody the server will refuse is a button that fails
    // after they press it. Preview and copy stay: neither changes anything.
    it("hides upload and delete from a reader who cannot use them", async () => {
        library([]);
        const view = mount(LibraryPane);
        await waitFor(() => expect(view.text()).toContain("airhorn.ogg"));
        // Absent, not explained. A reader who cannot upload does not need a paragraph
        // saying so — the table they can read is the whole answer.
        expect(view.text()).not.toContain("Add a sound");
        expect(view.button("Delete")).toBeNull();
        expect(view.button("Play")).not.toBeNull();
        expect(view.button("Copy id")).not.toBeNull();
    });

    it("offers upload and delete to somebody who can use them", async () => {
        library(["audio_upload", "audio_delete"]);
        const view = mount(LibraryPane);
        await waitFor(() => expect(view.button("Delete")).not.toBeNull());
        expect(view.text()).toContain("Add a sound");
    });

    // A permission read that fails must not hand out permissions.
    it("gives nothing away when the permissions cannot be read", async () => {
        mockInvoke({
            get_credential: () => {
                throw new Error("no credential");
            },
            list_audio_files: () => ({ items: [sound()], total: 1, page: 1, page_size: 8 }),
        });
        const view = mount(LibraryPane);
        await waitFor(() => expect(view.text()).toContain("airhorn.ogg"));
        expect(view.button("Delete")).toBeNull();
    });

    // Two renderings, one row list. A sound present in the table and missing from the cards
    // would be invisible on exactly one form factor, which is the kind of gap nobody notices
    // until a phone is the only thing to hand.
    it("renders every sound in both the table and the card list", async () => {
        library(["audio_delete"], [sound(), sound({ id: "snd_bell", original_filename: "bell.ogg" })], 2);
        const view = mount(LibraryPane);
        await waitFor(() => expect(view.host.querySelectorAll(".rad-datacard")).toHaveLength(2));
        expect(view.host.querySelectorAll(".rad-table tbody tr")).toHaveLength(2);

        const cardIds = [...view.host.querySelectorAll(".rad-datacard__id")].map((el) =>
            el.textContent?.trim(),
        );
        expect(cardIds.sort()).toEqual(["snd_airhorn", "snd_bell"]);
    });

    // The card promotes the id because that is what the jukebox command takes, and drops the
    // four context columns to one meta line rather than to four columns nobody can reach.
    it("gives the card the same actions as the row", async () => {
        library(["audio_delete"]);
        const view = mount(LibraryPane);
        await waitFor(() => expect(view.host.querySelector(".rad-datacard")).not.toBeNull());

        const card = view.host.querySelector(".rad-datacard__actions");
        expect(card?.querySelector('[aria-label^="Play"]')).not.toBeNull();
        expect(card?.querySelector('[aria-label^="Copy id"]')).not.toBeNull();
        expect(card?.querySelector('[aria-label^="Delete"]')).not.toBeNull();
    });

    it("keeps delete out of the card for a reader who cannot delete", async () => {
        library([]);
        const view = mount(LibraryPane);
        await waitFor(() => expect(view.host.querySelector(".rad-datacard")).not.toBeNull());

        const card = view.host.querySelector(".rad-datacard__actions");
        expect(card?.querySelector('[aria-label^="Delete"]')).toBeNull();
        expect(card?.querySelector('[aria-label^="Play"]')).not.toBeNull();
    });

    // An empty library and a search that matched nothing are different problems.
    it("distinguishes an empty library from a search that found nothing", async () => {
        library(["audio_delete"], [], 0);
        const view = mount(LibraryPane);
        await waitFor(() => expect(view.text()).toContain("No sounds yet"));
    });

    // The server reads `page` as an index. Asking for 1 returned nothing, so a freshly
    // uploaded pair of sounds showed as "2 sounds" over an empty table.
    it("asks for the first page as the server counts it", async () => {
        library(["audio_delete"]);
        const view = mount(LibraryPane);
        await waitFor(() => expect(view.text()).toContain("airhorn.ogg"));

        const call = invokeCalls().find((c) => c.cmd === "list_audio_files");
        expect((call?.args as { query: { page: number } }).query.page).toBe(0);
    });
});

describe("LibraryPane preview", () => {
    class StubAudio {
        static pauses = 0;
        src: string;
        constructor(src: string) {
            this.src = src;
        }
        addEventListener(): void {}
        async play(): Promise<void> {}
        pause(): void {
            StubAudio.pauses += 1;
        }
    }

    let original: unknown;

    beforeEach(() => {
        StubAudio.pauses = 0;
        original = globalThis.Audio;
        globalThis.Audio = StubAudio as unknown as typeof Audio;
        mockInvoke({
            get_credential: () => JSON.stringify({ allowed: ["audio_delete"] }),
            list_audio_files: () => ({
                items: [sound(), sound({ id: "snd_bell", original_filename: "bell.ogg" })],
                total: 40,
                page: 0,
                page_size: 8,
            }),
            get_audio_stream_url: () => "https://bvc.example.com/stream",
        });
    });

    afterEach(() => {
        globalThis.Audio = original as typeof Audio;
    });

    // A track started from a table has nowhere else to be stopped from.
    it("turns the play button into a stop button while it plays", async () => {
        const view = mount(LibraryPane);
        await waitFor(() => expect(view.button("Play airhorn.ogg")).not.toBeNull());

        view.button("Play airhorn.ogg")?.click();
        await waitFor(() => expect(view.button("Stop airhorn.ogg")).not.toBeNull());
        expect(view.button("Play airhorn.ogg")).toBeNull();
        // The row that is not playing keeps its play button.
        expect(view.button("Play bell.ogg")).not.toBeNull();
    });

    it("stops the track when its own button is pressed again", async () => {
        const view = mount(LibraryPane);
        await waitFor(() => expect(view.button("Play airhorn.ogg")).not.toBeNull());

        view.button("Play airhorn.ogg")?.click();
        await waitFor(() => expect(view.button("Stop airhorn.ogg")).not.toBeNull());
        view.button("Stop airhorn.ogg")?.click();
        await waitFor(() => expect(view.button("Play airhorn.ogg")).not.toBeNull());
        expect(StubAudio.pauses).toBe(1);
    });

    // The rows are replaced, so the button holding the only way to stop it goes with them.
    it("stops the track when the page changes", async () => {
        const view = mount(LibraryPane);
        await waitFor(() => expect(view.button("Play airhorn.ogg")).not.toBeNull());

        view.button("Play airhorn.ogg")?.click();
        await waitFor(() => expect(view.button("Stop airhorn.ogg")).not.toBeNull());

        const next = [...view.host.querySelectorAll<HTMLElement>(".rad-pager__pages button")].find(
            (b) => b.textContent?.trim() === "2",
        );
        expect(next).not.toBeUndefined();
        next?.click();

        await waitFor(() => expect(StubAudio.pauses).toBe(1));
        expect(view.button("Stop airhorn.ogg")).toBeNull();
    });
});

describe("WebSocketPane", () => {
    function server(running: boolean, extra: Record<string, (args: never) => unknown> = {}) {
        mockInvoke({
            is_websocket_running: () => running,
            websocket_clients: () => [],
            update_websocket_config: () => null,
            start_websocket_server: () => null,
            stop_websocket_server: () => null,
            generate_encryption_key: () => "a-token",
            bedrock_list_interfaces: () => [{ name: "wlan0", ip: "192.168.1.24", is_ipv4: true }],
            ...extra,
        });
    }

    // A refused bind used to be swallowed. The toggle went back to off with nothing said,
    // which reads as "the setting does not persist" and sends everybody looking in the
    // wrong place.
    it("says why a start failed", async () => {
        server(false, {
            start_websocket_server: () => {
                throw new Error("Address already in use");
            },
        });
        const view = mount(WebSocketPane);
        await waitFor(() => expect(view.text()).toContain("Stopped"));

        view.host
            .querySelector<HTMLElement>('[aria-label="Enable the WebSocket server"]')
            ?.click();
        await waitFor(() => expect(view.text()).toContain("Address already in use"));
    });

    // Nothing below the switch means anything while the server is off, and an address
    // shown for a server that is not listening is an address that does not work.
    it("hides the address and the token while the server is off", async () => {
        server(false);
        const view = mount(WebSocketPane);
        await waitFor(() => expect(view.text()).toContain("Stopped"));
        expect(view.text()).not.toContain("Access token");
        expect(view.text()).not.toContain("Connected clients");
    });

    it("shows the address once it is listening", async () => {
        server(true);
        const view = mount(WebSocketPane);
        await waitFor(() => expect(view.text()).toContain("Listening"));
        expect(view.text()).toContain("Access token");
        expect(view.text()).toContain("Connected clients");
    });

    it("says so when nothing has connected yet", async () => {
        server(true);
        const view = mount(WebSocketPane);
        await waitFor(() => expect(view.text()).toContain("Nothing is connected yet"));
    });
});
