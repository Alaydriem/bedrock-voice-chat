import { beforeEach, describe, expect, it, vi } from "vitest";
import { render, screen } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import { mockInvoke } from "../tauri";
import DevicesScreen from "../../components/setup/DevicesScreen.svelte";

vi.mock("@tauri-apps/plugin-os", () => ({ platform: () => "windows" }));

const DEVICES = {
    input: [{ display_name: "Headset", io: "InputDevice" }],
    output: [{ display_name: "Speakers", io: "OutputDevice" }],
};

// The screen embeds the device pickers, which ask the backend for the device list on
// mount. Left unmocked they report a failure of their own, and this screen's own
// speaker-failure alert is then one alert among two.
beforeEach(() => {
    mockInvoke({
        get_devices: () => DEVICES,
        get_audio_device: ({ io }: { io: string }) =>
            io === "InputDevice" ? DEVICES.input[0] : DEVICES.output[0],
    });
});

function props(overrides: Record<string, unknown> = {}) {
  return {
    step: 3,
    total: 3,
    inputLevel: 0,
    gateOpen: false,
    oncontinue: vi.fn(),
    ...overrides,
  };
}

describe("DevicesScreen", () => {
  it("asks the user to choose their devices", () => {
    render(DevicesScreen, { props: props() });
    expect(screen.getByText(/pick your microphone/i)).toBeInTheDocument();
  });

  // The whole reason the input level was built: a meter that moves is how someone
  // knows the microphone they picked is the one that works.
  it("confirms the chosen microphone is getting through", () => {
    render(DevicesScreen, { props: props({ inputLevel: 0.6, gateOpen: true }) });
    expect(screen.getByText(/we can hear you/i)).toBeInTheDocument();
  });

  it("says so when nothing is arriving from the microphone", () => {
    render(DevicesScreen, { props: props({ inputLevel: 0, gateOpen: false }) });
    expect(screen.getByText(/say something/i)).toBeInTheDocument();
  });

  /**
   * A meter that never started and a microphone picking up silence both draw a flat
   * meter, and telling those apart is the only thing this screen is for. Asking someone
   * to say something when nothing could ever arrive sends them looking for a fault in
   * their voice instead of in their device.
   */
  it("distinguishes a microphone it cannot open from a quiet one", () => {
    render(DevicesScreen, { props: props({ available: false }) });
    // One caption, not two. There was a mono-caps line under this saying the same thing again
    // — and because the two changed together, every state change was two chances to reflow.
    expect(screen.getByText(/cannot open that microphone/i)).toBeInTheDocument();
    expect(screen.queryByText(/say something/i)).not.toBeInTheDocument();
  });

  /**
   * The microphone half of this screen reports itself continuously; nothing arrives to
   * prove the speakers work. Without this button the screen verifies half of what it asks
   * the user to choose.
   */
  it("plays a chime through the chosen output device", async () => {
    const ontestspeaker = vi.fn().mockResolvedValue(true);
    render(DevicesScreen, { props: props({ ontestspeaker }) });
    await userEvent.click(screen.getByRole("button", { name: /test playback/i }));
    expect(ontestspeaker).toHaveBeenCalledOnce();
  });

  // The command resolves when the chime ends, so a second press mid-chime would overlap
  // two copies of it and make a working device sound broken.
  it("cannot be pressed twice while the chime is playing", async () => {
    let release: ((value: boolean) => void) | undefined;
    const ontestspeaker = vi.fn(() => new Promise<boolean>((r) => (release = r)));
    render(DevicesScreen, { props: props({ ontestspeaker }) });

    await userEvent.click(screen.getByRole("button", { name: /test playback/i }));
    const playing = screen.getByRole("button", { name: /playing/i });
    expect(playing).toBeDisabled();

    release?.(true);
  });

  it("says so when the chosen output device cannot be played", async () => {
    const ontestspeaker = vi.fn().mockResolvedValue(false);
    render(DevicesScreen, { props: props({ ontestspeaker }) });
    await userEvent.click(screen.getByRole("button", { name: /test playback/i }));
    expect(await screen.findByRole("alert")).toHaveTextContent(/could not play/i);
  });

  it("lets the user finish setup", async () => {
    const oncontinue = vi.fn();
    render(DevicesScreen, { props: props({ oncontinue }) });
    await userEvent.click(screen.getByRole("button", { name: /finish setup/i }));
    expect(oncontinue).toHaveBeenCalledOnce();
  });

  it("shows where the reader is in setup", () => {
    render(DevicesScreen, { props: props() });
    expect(screen.getByText("03 / 03")).toBeInTheDocument();
  });
});
