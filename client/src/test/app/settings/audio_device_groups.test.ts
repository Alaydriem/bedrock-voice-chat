import { describe, expect, it } from "vitest";
import { AudioDeviceGroups } from "../../../js/app/settings/AudioDeviceGroups";
import type { AudioDevice } from "../../../js/bindings/AudioDevice";

function device(display_name: string, host: "Asio" | "Wasapi"): AudioDevice {
    return {
        io: "InputDevice",
        id: display_name,
        name: display_name,
        host,
        stream_configs: [],
        display_name,
    } as unknown as AudioDevice;
}

describe("AudioDeviceGroups.of", () => {
    it("puts a heading above each host's devices", () => {
        const options = AudioDeviceGroups.of([
            device("Focusrite USB ASIO", "Asio"),
            device("Realtek", "Wasapi"),
            device("Blue Yeti", "Wasapi"),
        ]);
        expect(options).toEqual([
            { section: "WASAPI" },
            "Realtek",
            "Blue Yeti",
            { section: "ASIO" },
            "Focusrite USB ASIO",
        ]);
    });

    // WASAPI is what almost everyone is on. Leading with ASIO puts the specialist case
    // above the common one every time the menu opens.
    it("lists WASAPI first", () => {
        const options = AudioDeviceGroups.of([
            device("Focusrite USB ASIO", "Asio"),
            device("Realtek", "Wasapi"),
        ]);
        expect(options[0]).toEqual({ section: "WASAPI" });
    });

    // A heading with nothing under it is a claim about what is available.
    it("omits a host with no devices", () => {
        const options = AudioDeviceGroups.of([device("Realtek", "Wasapi")]);
        expect(options).not.toContainEqual({ section: "ASIO" });
    });

    // One host means the heading is telling you nothing you cannot already see.
    it("drops the headings entirely when every device shares a host", () => {
        const options = AudioDeviceGroups.of([
            device("Realtek", "Wasapi"),
            device("Blue Yeti", "Wasapi"),
        ]);
        expect(options).toEqual(["Realtek", "Blue Yeti"]);
    });

    it("returns nothing for no devices", () => {
        expect(AudioDeviceGroups.of([])).toEqual([]);
    });
});
