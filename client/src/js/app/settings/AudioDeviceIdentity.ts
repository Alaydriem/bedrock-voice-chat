import type { AudioDevice } from "../../bindings/AudioDevice";

/**
 * Which device a selection meant.
 *
 * A display name does not answer that. One interface is offered under every host that can
 * reach it — the same hardware under ASIO and under WASAPI is a different latency, not a
 * duplicate — and an ASIO device presents each of its channels under a single device id.
 * So the name is ambiguous in one direction and the id is ambiguous in the other, and only
 * the two together with the host and the direction name one endpoint.
 *
 * The configs are deliberately excluded. What comes back from `get_audio_device` is the
 * device as it was persisted, and its config list may no longer match a fresh enumeration;
 * an identity that included them would stop matching the device it names.
 */
export class AudioDeviceIdentity {
    /**
     * ASCII unit separator. No device name or driver id contains one, so no name can be
     * spelled to forge another device's key — which a punctuation separator allows: an id
     * of `a` with a name of `b c` and an id of `a b` with a name of `c` would collide.
     *
     * Safe in an attribute value, which a NUL is not: the HTML parser rewrites U+0000.
     */
    private static readonly SEPARATOR = "\u001F";

    static keyOf(device: AudioDevice): string {
        return [device.io, device.host, device.id, device.display_name].join(this.SEPARATOR);
    }

    /**
     * The list with any repeated identity removed, first occurrence kept.
     *
     * The backend collapses these where they are produced, which is where the fix
     * belongs. This is here because the cost of being wrong is out of all proportion to
     * the fault: a keyed `{#each}` given a duplicate key throws out of the render in a
     * release build as well as in dev, and it takes the whole settings screen down with
     * it — no device picker, no navigation, no way to reach any other pane.
     */
    static unique(devices: readonly AudioDevice[]): readonly AudioDevice[] {
        const seen = new Set<string>();
        return devices.filter((device) => {
            const key = this.keyOf(device);
            if (seen.has(key)) return false;
            seen.add(key);
            return true;
        });
    }

    /** The device a key names, or undefined once it has been unplugged. */
    static find(devices: readonly AudioDevice[], key: string): AudioDevice | undefined {
        return devices.find((device) => this.keyOf(device) === key);
    }
}
