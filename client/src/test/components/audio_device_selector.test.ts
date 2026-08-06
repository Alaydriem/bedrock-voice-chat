import { render, waitFor } from "@testing-library/svelte";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { mockInvoke, invokeCalls } from "../tauri";

function sentTo(cmd: string): Record<string, unknown> | undefined {
    return invokeCalls().find((call) => call.cmd === cmd)?.args as
        | Record<string, unknown>
        | undefined;
}
import AudioDeviceSelector from "../../components/audio/AudioDeviceSelector.svelte";

vi.mock("@tauri-apps/plugin-os", () => ({ platform: () => "windows" }));

function asio(displayName: string, sampleRate: number) {
    return {
        io: "InputDevice",
        id: "focusrite",
        name: "Focusrite USB ASIO",
        host: "Asio",
        display_name: displayName,
        stream_configs: [
            {
                channels: 1,
                sample_rate: sampleRate,
                sample_format: "f32",
                buffer_size_min: 0,
                buffer_size_max: 0,
            },
        ],
    };
}

const SPEAKERS = {
    io: "OutputDevice",
    id: "speakers",
    name: "Speakers",
    host: "Wasapi",
    display_name: "Speakers",
    stream_configs: [],
};

function devices(input: unknown[]) {
    mockInvoke({
        get_devices: () => ({ Asio: input, Wasapi: [SPEAKERS] }),
        get_audio_device: ({ io }: { io: string }) =>
            io === "InputDevice" ? input[0] : SPEAKERS,
        set_audio_device: () => null,
        change_audio_device: () => null,
    });
}

function mount() {
    const host = document.createElement("div");
    document.body.append(host);
    render(AudioDeviceSelector as never, { target: host } as never);
    return host;
}

beforeEach(() => {
    devices([asio("Focusrite USB ASIO Input 1", 48000)]);
});

describe("AudioDeviceSelector", () => {
    it("offers the devices the backend reported", async () => {
        const host = mount();
        await waitFor(() =>
            expect(host.querySelectorAll("option").length).toBeGreaterThan(0),
        );
        expect(host.textContent).toContain("Focusrite USB ASIO Input 1");
    });

    /**
     * The reported crash. An ASIO driver offering two rates on one channel produced two
     * entries with the same name; a keyed `{#each}` throws on a duplicate key in a
     * release build as well as in dev, which took down the entire settings screen —
     * the device list, the nav rail and every other pane with it.
     */
    it("survives two devices the driver gave the same name", async () => {
        devices([
            asio("Focusrite USB ASIO Input 1", 44100),
            asio("Focusrite USB ASIO Input 1", 48000),
        ]);

        const host = mount();

        await waitFor(() => expect(host.querySelector("select")).not.toBeNull());
        expect(host.textContent).toContain("Focusrite USB ASIO Input 1");
    });

    it("offers a name the driver repeated exactly once", async () => {
        devices([
            asio("Focusrite USB ASIO Input 1", 44100),
            asio("Focusrite USB ASIO Input 1", 48000),
        ]);

        const host = mount();

        await waitFor(() =>
            expect(host.querySelectorAll("option").length).toBeGreaterThan(0),
        );
        const inputs = [...(host.querySelectorAll("select")[0]?.options ?? [])];
        expect(inputs.map((o) => o.textContent?.trim())).toEqual([
            "Focusrite USB ASIO Input 1",
        ]);
    });

    /**
     * Two hosts reaching one interface is a real choice — a different latency, not a
     * duplicate — so picking one has to send the host the user picked. Selecting by name
     * sent whichever sorted first.
     */
    it("sends the host that was picked, not the first with that name", async () => {
        const underAsio = { ...asio("Focusrite USB", 48000), host: "Asio" };
        const underWasapi = { ...asio("Focusrite USB", 48000), host: "Wasapi" };
        devices([underAsio, underWasapi]);

        const host = mount();
        await waitFor(() =>
            expect(host.querySelectorAll("select")[0]?.options.length).toBe(2),
        );

        const select = host.querySelectorAll("select")[0] as HTMLSelectElement;
        select.value = select.options[1].value;
        select.dispatchEvent(new Event("change", { bubbles: true }));

        await waitFor(() => expect(sentTo("set_audio_device")).not.toBeUndefined());
        const sent = sentTo("set_audio_device") as { device: { host: string } };
        expect(sent.device.host).toBe("Wasapi");
    });
});
