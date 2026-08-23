import { render } from "@testing-library/svelte";
import { describe, expect, it, vi } from "vitest";
import type { RecordingRow } from "../../js/app/settings/RecordingRow";
import type { RecordingTrack } from "../../js/bindings/RecordingTrack";

const { default: RecordingDetail } = await import(
    "../../components/settings/RecordingDetail.svelte"
);

const row: RecordingRow = {
    id: "0191f3c2-0000-7000-8000-000000000000",
    name: "Nether run",
    unnamed: false,
    recorded: "2026-07-28 21:14",
    recordedAt: 1_753_733_640_000,
    length: "1:42:08",
    players: 4,
    size: "412 MB",
    bytes: 412_000_000,
    exportable: true,
};

const tracks: readonly RecordingTrack[] = [
    { keys: ["minecraft:Alaydriem"], display: "Alaydriem", kind: "Own" },
    { keys: ["minecraft:Petra"], display: "Petra", kind: "Player" },
];

function mount(props: Record<string, unknown> = {}) {
    const host = document.createElement("div");
    document.body.append(host);
    render(RecordingDetail as never, {
        target: host,
        props: {
            row,
            tracks,
            chosen: new Set(tracks.map((track) => track.display)),
            progress: null,
            spatial: true,
            onback: () => {},
            ontoggle: () => {},
            onall: () => {},
            onnone: () => {},
            onspatial: () => {},
            onexport: () => {},
            onrename: () => {},
            ondelete: () => {},
            ...props,
        },
    } as never);
    return host;
}

// The toggle is a role=switch button, not a checkbox, so it is found by its accessible name.
function spatialToggle(host: HTMLElement): HTMLButtonElement {
    const toggle = host.querySelector<HTMLButtonElement>(
        '[role="switch"][aria-label="Mix in the spatial positions"]',
    );
    if (!toggle) throw new Error("the spatial toggle is not on the screen");
    return toggle;
}

function isOn(toggle: HTMLButtonElement): boolean {
    return toggle.getAttribute("aria-checked") === "true";
}

describe("RecordingDetail spatial toggle", () => {
    // It shipped disabled and inert. The point of GH-118 is that it is neither.
    it("is not disabled when no render is running", () => {
        const host = mount();

        expect(spatialToggle(host).disabled).toBe(false);
    });

    it("shows the state it was given", () => {
        expect(isOn(spatialToggle(mount({ spatial: true })))).toBe(true);
        expect(isOn(spatialToggle(mount({ spatial: false })))).toBe(false);
    });

    it("reports a change to its caller", async () => {
        const onspatial = vi.fn();
        const host = mount({ spatial: true, onspatial });

        spatialToggle(host).click();

        expect(onspatial).toHaveBeenCalledWith(false);
    });

    // Changing the curve part way through a run would leave a folder of tracks rendered two
    // different ways, with nothing in the output saying which was which.
    it("is locked while a render is in progress", () => {
        const host = mount({
            progress: { track: "Petra", index: 1, total: 2 },
        });

        expect(spatialToggle(host).disabled).toBe(true);
    });
});
