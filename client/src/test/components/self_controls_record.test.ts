import { render } from "@testing-library/svelte";
import { beforeEach, describe, expect, it, vi } from "vitest";
import "../tauri";
import { ConstantLevelSource } from "../../radial/core/sources/LevelSource";
import type { SelfSnapshot } from "../../radial/core/controllers/SelfState";
import type { SelfController } from "../../js/app/dashboard/SelfController";

let platformName = "windows";
vi.mock("@tauri-apps/plugin-os", () => ({ platform: () => platformName }));

const { default: SelfControls } = await import(
    "../../components/dashboard/SelfControls.svelte"
);

const state: SelfSnapshot = {
    muted: false,
    deafened: false,
    recording: false,
    mode: "activated",
    holding: false,
    transmitting: true,
    recordAllowed: true,
    captureAvailable: true,
};

function mount() {
    const host = document.createElement("div");
    document.body.append(host);
    const controller = {
        micSource: new ConstantLevelSource(0),
        elapsed: () => "00:00",
        pressRecord: vi.fn(),
        hold: vi.fn(),
    } as unknown as SelfController;
    render(SelfControls as never, {
        target: host,
        props: {
            controller,
            selfState: state,
            name: "Alaydriem",
            onmute: vi.fn(),
            ondeafen: vi.fn(),
            onidentity: vi.fn(),
        },
    } as never);
    return host;
}

describe("SelfControls record button by platform", () => {
    beforeEach(() => {
        platformName = "windows";
    });

    it("offers the record button on a desktop platform", () => {
        expect(mount().querySelector(".rad-self__btn--record")).not.toBeNull();
    });

    // The pill is chosen by a container width, and a tablet is wider than the phone
    // breakpoint — so without a platform check iPadOS renders the desktop pill and every
    // control on it, recording included.
    it("omits the record button on iOS, where the desktop pill is what a tablet renders", () => {
        platformName = "ios";

        expect(mount().querySelector(".rad-self__btn--record")).toBeNull();
    });

    it("omits the record button on Android", () => {
        platformName = "android";

        expect(mount().querySelector(".rad-self__btn--record")).toBeNull();
    });
});
